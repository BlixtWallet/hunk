use std::collections::BTreeMap;
use std::env;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use semver::Version;
use serde::{Deserialize, Serialize};

pub const DEFAULT_UPDATE_MANIFEST_URL: &str = "https://updates.hunk.dev/stable.json";
pub const UPDATE_EXPLANATION_ENV_VAR: &str = "HUNK_UPDATE_EXPLANATION";
pub const UPDATE_MANIFEST_URL_ENV_VAR: &str = "HUNK_UPDATE_MANIFEST_URL";

const UPDATE_HTTP_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssetFormat {
    App,
    Wix,
    Tarball,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub url: String,
    pub signature: String,
    pub format: AssetFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub version: String,
    #[serde(default)]
    pub pub_date: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub platforms: BTreeMap<String, ReleaseAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallSource {
    SelfManaged,
    PackageManaged { explanation: String },
}

impl InstallSource {
    pub fn explanation(&self) -> Option<&str> {
        match self {
            Self::SelfManaged => None,
            Self::PackageManaged { explanation } => Some(explanation.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableUpdate {
    pub manifest_url: String,
    pub version: String,
    pub pub_date: Option<String>,
    pub notes: Option<String>,
    pub target: String,
    pub asset: ReleaseAsset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheckResult {
    UpToDate { version: String },
    UpdateAvailable(AvailableUpdate),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    DisabledByInstallSource {
        explanation: String,
    },
    UpToDate {
        version: String,
        checked_at_unix_ms: i64,
    },
    UpdateAvailable(AvailableUpdate),
    Error(String),
}

pub fn resolve_manifest_url() -> String {
    env::var(UPDATE_MANIFEST_URL_ENV_VAR)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_UPDATE_MANIFEST_URL.to_string())
}

pub fn detect_install_source() -> InstallSource {
    install_source_from_explanation(env::var(UPDATE_EXPLANATION_ENV_VAR).ok().as_deref())
}

pub fn install_source_from_explanation(explanation: Option<&str>) -> InstallSource {
    explanation
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| InstallSource::PackageManaged {
            explanation: value.to_string(),
        })
        .unwrap_or(InstallSource::SelfManaged)
}

pub fn current_update_target() -> Result<&'static str> {
    match (env::consts::OS, env::consts::ARCH) {
        ("macos", "aarch64") => Ok("macos-aarch64"),
        ("macos", "x86_64") => Ok("macos-x86_64"),
        ("windows", "x86_64") => Ok("windows-x86_64"),
        ("windows", "aarch64") => Ok("windows-aarch64"),
        ("linux", "x86_64") => Ok("linux-x86_64"),
        ("linux", "aarch64") => Ok("linux-aarch64"),
        (os, arch) => bail!("unsupported update target: {os}-{arch}"),
    }
}

pub fn check_for_updates(manifest_url: &str, current_version: &str) -> Result<UpdateCheckResult> {
    let manifest = reqwest::blocking::Client::builder()
        .timeout(UPDATE_HTTP_TIMEOUT)
        .user_agent(format!("hunk-updater/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .context("failed to create updater HTTP client")?
        .get(manifest_url)
        .send()
        .with_context(|| format!("failed to fetch update manifest from {manifest_url}"))?
        .error_for_status()
        .with_context(|| format!("update manifest request failed for {manifest_url}"))?
        .json::<ReleaseManifest>()
        .with_context(|| format!("failed to parse update manifest from {manifest_url}"))?;

    evaluate_manifest(
        manifest_url,
        current_version,
        current_update_target()?,
        manifest,
    )
}

pub fn evaluate_manifest(
    manifest_url: &str,
    current_version: &str,
    target: &str,
    manifest: ReleaseManifest,
) -> Result<UpdateCheckResult> {
    let current = parse_stable_version(current_version)
        .with_context(|| format!("invalid current app version `{current_version}`"))?;
    let latest = parse_stable_version(manifest.version.as_str())
        .with_context(|| format!("invalid update manifest version `{}`", manifest.version))?;

    if latest <= current {
        return Ok(UpdateCheckResult::UpToDate {
            version: manifest.version,
        });
    }

    let asset = manifest
        .platforms
        .get(target)
        .cloned()
        .ok_or_else(|| anyhow!("update manifest does not contain platform asset `{target}`"))?;

    Ok(UpdateCheckResult::UpdateAvailable(AvailableUpdate {
        manifest_url: manifest_url.to_string(),
        version: manifest.version,
        pub_date: manifest.pub_date,
        notes: manifest.notes,
        target: target.to_string(),
        asset,
    }))
}

fn parse_stable_version(raw: &str) -> Result<Version> {
    let version = Version::parse(raw)?;
    if !version.pre.is_empty() {
        bail!("prerelease versions are not supported");
    }
    Ok(version)
}
