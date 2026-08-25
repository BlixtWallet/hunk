use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, bail};
use hunk_domain::config::{
    AppConfig, ConfigStore, ForgeCredentialConfig, ForgeCredentialKind as ConfigCredentialKind,
    ForgeRepoCredentialBindingConfig, ReviewProviderKind,
};
use hunk_forge::{
    ForgeCredentialKind, ForgeCredentialMetadata, ForgeProvider, ForgeRepoCredentialBinding,
    ForgeReviewClient, ForgeReviewOutcome, ForgeReviewWorkspace, ForgeSecretStore, GitHubAuthMode,
    GitHubDeviceAuthorization, GitHubDeviceFlowPoll, GitHubDeviceFlowService, GitHubReviewClient,
    KeyringForgeSecretStore, OpenReviewQuery, OpenReviewSummary, find_or_create_review,
    github_auth_mode_for_host, resolve_credential_for_repo, resolve_review_workspace,
};

const GITHUB_TOKEN_ENV_KEYS: &[&str] = &["HUNK_GITHUB_TOKEN", "GITHUB_TOKEN"];
const GITLAB_TOKEN_ENV_KEYS: &[&str] = &["HUNK_GITLAB_TOKEN", "GITLAB_TOKEN"];
const HUNK_GITHUB_OAUTH_CLIENT_ID: &str = "Ov23liecmGTDOJDVpP5c";

#[derive(Debug)]
pub struct ForgeSnapshotPayload {
    pub workspace: ForgeReviewWorkspace,
    pub token: Option<String>,
    pub account_label: String,
    pub review: Option<OpenReviewSummary>,
}

impl ForgeSnapshotPayload {
    pub fn authenticated(&self) -> bool {
        self.token.is_some()
    }

    pub fn auth_mode(&self) -> &'static str {
        match self.workspace.base_repo.provider {
            ForgeProvider::GitHub
                if github_auth_mode_for_host(self.workspace.base_repo.host.as_str())
                    == GitHubAuthMode::DeviceFlow =>
            {
                "device"
            }
            _ => "token",
        }
    }
}

pub fn load_forge_snapshot(
    repo_root: &std::path::Path,
    branch: &str,
) -> Result<ForgeSnapshotPayload> {
    let config = load_config()?;
    let workspace = resolve_review_workspace(
        repo_root,
        branch,
        config.review_provider_mappings.as_slice(),
    )?;
    let (token, account_label) = resolve_token(&workspace.base_repo, &config)?;
    let review = token
        .as_deref()
        .map(|token| {
            ForgeReviewClient::new(&workspace.base_repo, token)?.find_branch_review(
                &OpenReviewQuery {
                    base_repo: workspace.base_repo.clone(),
                    head_repo: workspace.head_repo.clone(),
                    source_branch: workspace.source_branch.clone(),
                    target_branch: None,
                },
            )
        })
        .transpose()?
        .flatten();

    Ok(ForgeSnapshotPayload {
        workspace,
        token,
        account_label,
        review,
    })
}

pub fn save_forge_token(
    workspace: ForgeReviewWorkspace,
    token: &str,
    kind: ForgeCredentialKind,
) -> Result<ForgeSnapshotPayload> {
    let token = token.trim();
    if token.is_empty() {
        bail!("access token is required");
    }

    let client = ForgeReviewClient::new(&workspace.base_repo, token)?;
    let review = client.find_branch_review(&OpenReviewQuery {
        base_repo: workspace.base_repo.clone(),
        head_repo: workspace.head_repo.clone(),
        source_branch: workspace.source_branch.clone(),
        target_branch: None,
    })?;
    let (account_label, account_login) = match workspace.base_repo.provider {
        ForgeProvider::GitHub => {
            let account =
                GitHubReviewClient::for_repo(&workspace.base_repo, token)?.current_user()?;
            (account.display_label, Some(account.login))
        }
        ForgeProvider::GitLab => (workspace.base_repo.path.clone(), None),
    };

    let store = ConfigStore::new()?;
    let mut config = store.load_or_create_default()?;
    let credential_id = upsert_repo_credential(
        &mut config,
        &workspace,
        kind,
        account_label.as_str(),
        account_login.clone(),
    );
    KeyringForgeSecretStore
        .save_secret(credential_id.as_str(), token)
        .context("failed to save forge access token")?;
    store.save(&config)?;

    Ok(ForgeSnapshotPayload {
        workspace,
        token: Some(token.to_string()),
        account_label,
        review,
    })
}

pub fn create_or_find_review(
    workspace: &ForgeReviewWorkspace,
    token: &str,
    target_branch: &str,
    title: &str,
    body: Option<String>,
    draft: bool,
) -> Result<ForgeReviewOutcome> {
    let client = ForgeReviewClient::new(&workspace.base_repo, token)?;
    find_or_create_review(&client, workspace, target_branch, title, body, draft)
}

pub fn run_github_device_flow(
    workspace: ForgeReviewWorkspace,
    epoch: i32,
    current_epoch: Arc<AtomicI32>,
    mut on_started: impl FnMut(Result<GitHubDeviceAuthorization, String>),
) -> Option<Result<ForgeSnapshotPayload, String>> {
    let service = match GitHubDeviceFlowService::new(HUNK_GITHUB_OAUTH_CLIENT_ID) {
        Ok(service) => service,
        Err(error) => {
            on_started(Err(format!("{error:#}")));
            return None;
        }
    };
    let authorization = match service.start_device_flow() {
        Ok(authorization) => authorization,
        Err(error) => {
            on_started(Err(format!("{error:#}")));
            return None;
        }
    };
    on_started(Ok(authorization.clone()));

    let started_at = Instant::now();
    let expires_after = Duration::from_secs(authorization.expires_in_secs);
    let mut poll_interval = Duration::from_secs(authorization.interval_secs.max(1));
    loop {
        if current_epoch.load(Ordering::Acquire) != epoch {
            return None;
        }
        if started_at.elapsed() >= expires_after {
            return Some(Err(
                "GitHub authorization expired; start sign-in again".to_owned()
            ));
        }
        std::thread::sleep(poll_interval);
        if current_epoch.load(Ordering::Acquire) != epoch {
            return None;
        }
        match service.poll_device_flow_token(authorization.device_code.as_str()) {
            Ok(GitHubDeviceFlowPoll::AuthorizationPending) => {}
            Ok(GitHubDeviceFlowPoll::SlowDown) => poll_interval += Duration::from_secs(5),
            Ok(GitHubDeviceFlowPoll::Complete(token)) => {
                return Some(
                    save_forge_token(
                        workspace,
                        token.access_token.as_str(),
                        ForgeCredentialKind::GitHubComSession,
                    )
                    .map_err(|error| format!("{error:#}")),
                );
            }
            Ok(GitHubDeviceFlowPoll::AccessDenied(description)) => {
                return Some(Err(format!(
                    "GitHub authorization was denied: {description}"
                )));
            }
            Ok(GitHubDeviceFlowPoll::ExpiredToken) => {
                return Some(Err(
                    "GitHub authorization expired; start sign-in again".to_owned()
                ));
            }
            Err(error) => return Some(Err(format!("{error:#}"))),
        }
    }
}

pub fn provider_label(provider: ForgeProvider) -> &'static str {
    match provider {
        ForgeProvider::GitHub => "GitHub",
        ForgeProvider::GitLab => "GitLab",
    }
}

pub fn review_kind_label(provider: ForgeProvider) -> &'static str {
    match provider {
        ForgeProvider::GitHub => "Pull Request",
        ForgeProvider::GitLab => "Merge Request",
    }
}

pub fn review_short_label(provider: ForgeProvider) -> &'static str {
    match provider {
        ForgeProvider::GitHub => "PR",
        ForgeProvider::GitLab => "MR",
    }
}

pub fn review_state_label(review: &OpenReviewSummary) -> &'static str {
    use hunk_forge::ForgeReviewState;
    match review.state {
        ForgeReviewState::Open if review.draft => "Draft",
        ForgeReviewState::Open => "Open",
        ForgeReviewState::Closed => "Closed",
        ForgeReviewState::Merged => "Merged",
    }
}

fn load_config() -> Result<AppConfig> {
    ConfigStore::new()?.load_or_create_default()
}

fn resolve_token(
    repo: &hunk_forge::ForgeRepoRef,
    config: &AppConfig,
) -> Result<(Option<String>, String)> {
    let credentials = config
        .forge_credentials
        .iter()
        .map(|credential| ForgeCredentialMetadata {
            id: credential.id.clone(),
            provider: forge_provider(credential.provider),
            host: credential.host.clone(),
            kind: forge_credential_kind(credential.kind),
            account_label: credential.account_label.clone(),
            account_login: credential.account_login.clone(),
            is_default_for_host: credential.is_default_for_host,
        })
        .collect::<Vec<_>>();
    let bindings = config
        .forge_repo_credential_bindings
        .iter()
        .map(|binding| ForgeRepoCredentialBinding {
            provider: forge_provider(binding.provider),
            host: binding.host.clone(),
            repo_path: binding.repo_path.clone(),
            credential_id: binding.credential_id.clone(),
        })
        .collect::<Vec<_>>();

    if let Some(resolved) = resolve_credential_for_repo(repo, &credentials, &bindings) {
        let credential = credentials
            .iter()
            .find(|credential| credential.id == resolved.credential_id)
            .context("resolved forge credential metadata is missing")?;
        let token = KeyringForgeSecretStore
            .load_secret(resolved.credential_id.as_str())
            .with_context(|| {
                format!(
                    "failed to load the saved {} credential",
                    provider_label(repo.provider)
                )
            })?;
        return Ok((token, credential.account_label.clone()));
    }

    let host_has_credentials = credentials
        .iter()
        .any(|credential| credential.provider == repo.provider && credential.host == repo.host);
    if host_has_credentials {
        return Ok((None, String::new()));
    }

    let token = token_env_keys(repo.provider)
        .iter()
        .find_map(|key| std::env::var(key).ok())
        .map(|token| token.trim().to_string())
        .filter(|token| !token.is_empty());
    let account_label = token
        .as_ref()
        .map(|_| "Environment credential".to_string())
        .unwrap_or_default();
    Ok((token, account_label))
}

fn upsert_repo_credential(
    config: &mut AppConfig,
    workspace: &ForgeReviewWorkspace,
    kind: ForgeCredentialKind,
    account_label: &str,
    account_login: Option<String>,
) -> String {
    let repo = &workspace.base_repo;
    let provider = review_provider(repo.provider);
    let existing_credential_id = config
        .forge_repo_credential_bindings
        .iter()
        .find(|binding| {
            binding.provider == provider
                && binding.host == repo.host
                && binding.repo_path == repo.path
        })
        .map(|binding| binding.credential_id.clone())
        .filter(|credential_id| {
            config
                .forge_credentials
                .iter()
                .any(|credential| credential.id == *credential_id)
        });
    let credential_id = existing_credential_id.unwrap_or_else(|| next_credential_id(repo));
    let host_has_default = config.forge_credentials.iter().any(|credential| {
        credential.provider == provider
            && credential.host == repo.host
            && credential.is_default_for_host
            && credential.id != credential_id
    });

    if let Some(credential) = config
        .forge_credentials
        .iter_mut()
        .find(|credential| credential.id == credential_id)
    {
        credential.provider = provider;
        credential.host = repo.host.clone();
        credential.kind = config_credential_kind(kind);
        credential.account_label = account_label.to_string();
        credential.account_login = account_login;
        credential.is_default_for_host = !host_has_default;
    } else {
        config.forge_credentials.push(ForgeCredentialConfig {
            id: credential_id.clone(),
            provider,
            host: repo.host.clone(),
            kind: config_credential_kind(kind),
            account_label: account_label.to_string(),
            account_login,
            is_default_for_host: !host_has_default,
        });
    }

    config.forge_repo_credential_bindings.retain(|binding| {
        !(binding.provider == provider
            && binding.host == repo.host
            && binding.repo_path == repo.path)
    });
    config
        .forge_repo_credential_bindings
        .push(ForgeRepoCredentialBindingConfig {
            provider,
            host: repo.host.clone(),
            repo_path: repo.path.clone(),
            credential_id: credential_id.clone(),
        });
    credential_id
}

fn next_credential_id(repo: &hunk_forge::ForgeRepoRef) -> String {
    let provider = match repo.provider {
        ForgeProvider::GitHub => "github",
        ForgeProvider::GitLab => "gitlab",
    };
    let host = id_fragment(repo.host.as_str());
    let path = id_fragment(repo.path.as_str());
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{provider}-{host}-{path}-{nonce}")
}

fn id_fragment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn token_env_keys(provider: ForgeProvider) -> &'static [&'static str] {
    match provider {
        ForgeProvider::GitHub => GITHUB_TOKEN_ENV_KEYS,
        ForgeProvider::GitLab => GITLAB_TOKEN_ENV_KEYS,
    }
}

fn forge_provider(provider: ReviewProviderKind) -> ForgeProvider {
    match provider {
        ReviewProviderKind::GitHub => ForgeProvider::GitHub,
        ReviewProviderKind::GitLab => ForgeProvider::GitLab,
    }
}

fn review_provider(provider: ForgeProvider) -> ReviewProviderKind {
    match provider {
        ForgeProvider::GitHub => ReviewProviderKind::GitHub,
        ForgeProvider::GitLab => ReviewProviderKind::GitLab,
    }
}

fn forge_credential_kind(kind: ConfigCredentialKind) -> ForgeCredentialKind {
    match kind {
        ConfigCredentialKind::PersonalAccessToken => ForgeCredentialKind::PersonalAccessToken,
        ConfigCredentialKind::GitHubComSession => ForgeCredentialKind::GitHubComSession,
    }
}

fn config_credential_kind(kind: ForgeCredentialKind) -> ConfigCredentialKind {
    match kind {
        ForgeCredentialKind::PersonalAccessToken => ConfigCredentialKind::PersonalAccessToken,
        ForgeCredentialKind::GitHubComSession => ConfigCredentialKind::GitHubComSession,
    }
}
