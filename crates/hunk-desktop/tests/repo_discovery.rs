#[path = "../src/app/repo_discovery.rs"]
mod repo_discovery;

use anyhow::anyhow;

#[test]
fn missing_repository_error_detection_is_case_insensitive() {
    let err = anyhow!("failed to discover Git repository from /tmp/hunk");

    assert!(repo_discovery::is_missing_repository_error(&err));
}

#[test]
fn missing_repository_error_detection_matches_non_repo_messages() {
    let err = anyhow!("not a git repository");

    assert!(repo_discovery::is_missing_repository_error(&err));
}
