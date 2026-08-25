pub mod auth;
mod client;
pub mod github;
pub mod github_auth;
pub mod gitlab;
mod keyring_store;
mod models;
mod workspace;

pub use auth::{
    ForgeCredentialKind, ForgeCredentialMetadata, ForgeCredentialResolution,
    ForgeRepoCredentialBinding, ForgeSecretStore, ResolvedForgeCredential,
    resolve_credential_for_repo,
};
pub use client::ForgeReviewClient;
pub use github::{GitHubAuthenticatedAccount, GitHubReviewClient};
pub use github_auth::{
    GitHubAuthMode, GitHubAuthScopes, GitHubDeviceAuthorization, GitHubDeviceFlowPoll,
    GitHubDeviceFlowService, GitHubOAuthAppConfig, GitHubOAuthToken, github_auth_mode_for_host,
};
pub use gitlab::GitLabReviewClient;
pub use keyring_store::KeyringForgeSecretStore;
pub use models::{
    CreateReviewInput, CreateReviewResult, ForgeProvider, ForgeRepoRef, ForgeReviewState,
    OpenReviewQuery, OpenReviewSummary,
};
pub use workspace::{
    ForgeReviewApi, ForgeReviewOutcome, ForgeReviewWorkspace, find_or_create_review,
    resolve_review_workspace,
};
