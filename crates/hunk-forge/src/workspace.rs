use std::path::Path;

use anyhow::{Result, bail};
use hunk_git::branch::{
    ReviewRemote, review_remote_for_branch_with_provider_map,
    review_remote_for_named_remote_with_provider_map,
};
use hunk_git::compare::resolve_default_base_branch_name;
use hunk_git::config::ReviewProviderMapping;

use crate::{
    CreateReviewInput, CreateReviewResult, ForgeRepoRef, ForgeReviewClient, OpenReviewQuery,
    OpenReviewSummary,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeReviewWorkspace {
    pub review_remote: ReviewRemote,
    pub base_repo: ForgeRepoRef,
    pub head_repo: ForgeRepoRef,
    pub source_branch: String,
    pub target_branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeReviewOutcome {
    pub review: OpenReviewSummary,
    pub existed: bool,
}

pub trait ForgeReviewApi {
    fn find_open_review(&self, query: &OpenReviewQuery) -> Result<Option<OpenReviewSummary>>;

    fn find_branch_review(&self, query: &OpenReviewQuery) -> Result<Option<OpenReviewSummary>>;

    fn create_review(&self, input: &CreateReviewInput) -> Result<CreateReviewResult>;
}

impl ForgeReviewApi for ForgeReviewClient {
    fn find_open_review(&self, query: &OpenReviewQuery) -> Result<Option<OpenReviewSummary>> {
        self.find_open_review(query)
    }

    fn find_branch_review(&self, query: &OpenReviewQuery) -> Result<Option<OpenReviewSummary>> {
        self.find_branch_review(query)
    }

    fn create_review(&self, input: &CreateReviewInput) -> Result<CreateReviewResult> {
        self.create_review(input)
    }
}

pub fn resolve_review_workspace(
    repo_root: &Path,
    source_branch: &str,
    provider_mappings: &[ReviewProviderMapping],
) -> Result<ForgeReviewWorkspace> {
    let source_branch = source_branch.trim();
    if source_branch.is_empty() || matches!(source_branch, "detached" | "unknown") {
        bail!("cannot resolve a review without an active branch");
    }

    let head_remote =
        review_remote_for_branch_with_provider_map(repo_root, source_branch, provider_mappings)?
            .ok_or_else(|| anyhow::anyhow!("no review remote found for the active branch"))?;
    let head_repo = ForgeRepoRef::try_from(&head_remote)?;
    let base_remote =
        review_remote_for_named_remote_with_provider_map(repo_root, "upstream", provider_mappings)?
            .filter(|candidate| {
                candidate.provider == head_remote.provider
                    && candidate.repository_path != head_remote.repository_path
            })
            .unwrap_or_else(|| head_remote.clone());
    let base_repo = ForgeRepoRef::try_from(&base_remote)?;
    let target_branch = preferred_review_base_branch(repo_root, source_branch);

    Ok(ForgeReviewWorkspace {
        review_remote: base_remote,
        base_repo,
        head_repo,
        source_branch: source_branch.to_string(),
        target_branch,
    })
}

pub fn find_or_create_review(
    client: &impl ForgeReviewApi,
    workspace: &ForgeReviewWorkspace,
    target_branch: &str,
    title: &str,
    body: Option<String>,
    draft: bool,
) -> Result<ForgeReviewOutcome> {
    let target_branch = target_branch.trim();
    if target_branch.is_empty() {
        bail!("base branch is required");
    }
    if target_branch == workspace.source_branch {
        bail!("base branch must differ from the source branch");
    }
    let title = title.trim();
    if title.is_empty() {
        bail!("review title is required");
    }

    if let Some(review) = client.find_open_review(&OpenReviewQuery {
        base_repo: workspace.base_repo.clone(),
        head_repo: workspace.head_repo.clone(),
        source_branch: workspace.source_branch.clone(),
        target_branch: Some(target_branch.to_string()),
    })? {
        return Ok(ForgeReviewOutcome {
            review,
            existed: true,
        });
    }

    let result = client.create_review(&CreateReviewInput {
        base_repo: workspace.base_repo.clone(),
        head_repo: workspace.head_repo.clone(),
        source_branch: workspace.source_branch.clone(),
        target_branch: target_branch.to_string(),
        title: title.to_string(),
        body: body.filter(|body| !body.trim().is_empty()),
        draft,
    })?;
    Ok(ForgeReviewOutcome {
        review: result.review,
        existed: false,
    })
}

fn preferred_review_base_branch(repo_root: &Path, source_branch: &str) -> String {
    let resolved = resolve_default_base_branch_name(repo_root)
        .ok()
        .flatten()
        .filter(|candidate| !candidate.trim().is_empty())
        .unwrap_or_else(|| "main".to_string());
    if resolved != source_branch {
        return resolved;
    }
    if source_branch != "main" {
        return "main".to_string();
    }
    "master".to_string()
}
