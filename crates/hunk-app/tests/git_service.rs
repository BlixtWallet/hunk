use std::fs;
use std::path::Path;

use git2::{Repository, Signature};
use hunk_app::git::{
    GitWorkspaceRefreshRequest, SnapshotRefreshBehavior, SnapshotRefreshPriority,
    SnapshotRefreshRequest, SnapshotRefreshResult, load_git_workspace_refresh,
    load_snapshot_refresh,
};

#[test]
fn refresh_requests_merge_to_the_most_urgent_behavior() {
    let merged = SnapshotRefreshRequest::background().merge(SnapshotRefreshRequest::user(true));

    assert!(merged.force);
    assert_eq!(merged.priority, SnapshotRefreshPriority::UserInitiated);
    assert_eq!(merged.behavior, SnapshotRefreshBehavior::RefreshWorkingCopy);
}

#[test]
fn queued_git_refreshes_merge_only_for_the_same_root() {
    let first_root = Path::new("/repo/first").to_path_buf();
    let second_root = Path::new("/repo/second").to_path_buf();

    let merged = GitWorkspaceRefreshRequest::new(first_root.clone(), false)
        .merge(GitWorkspaceRefreshRequest::new(first_root.clone(), true));
    assert_eq!(merged.root, first_root);
    assert!(merged.refresh_recent_commits);

    let replaced = merged.merge(GitWorkspaceRefreshRequest::new(second_root.clone(), false));
    assert_eq!(replaced.root, second_root);
    assert!(!replaced.refresh_recent_commits);
}

#[test]
fn headless_loaders_return_owned_snapshots_and_detect_unchanged_state() {
    let temp = tempfile::tempdir().expect("temporary repository should be created");
    initialize_repository(temp.path());
    fs::write(temp.path().join("pending.txt"), "pending\n")
        .expect("working-copy file should be written");

    let initial = load_git_workspace_refresh(temp.path(), None)
        .expect("initial Git workspace refresh should load");
    let workflow = initial
        .workflow
        .as_ref()
        .expect("initial refresh should include a workflow snapshot");
    assert!(workflow.files.iter().any(|file| file.path == "pending.txt"));

    let unchanged = load_git_workspace_refresh(temp.path(), Some(&initial.fingerprint))
        .expect("unchanged Git workspace refresh should load");
    assert!(unchanged.workflow.is_none());

    let snapshot = load_snapshot_refresh(
        temp.path(),
        None,
        SnapshotRefreshRequest::background(),
        true,
    )
    .expect("read-only snapshot refresh should load");
    assert!(matches!(
        snapshot,
        SnapshotRefreshResult::Loaded {
            loaded_without_refresh: true,
            ..
        }
    ));
}

fn initialize_repository(root: &Path) {
    let repo = Repository::init(root).expect("repository should initialize");
    fs::write(root.join("README.md"), "# fixture\n").expect("fixture file should be written");

    let mut index = repo.index().expect("repository index should open");
    index
        .add_path(Path::new("README.md"))
        .expect("fixture file should be staged");
    index.write().expect("repository index should be written");
    let tree_id = index.write_tree().expect("fixture tree should be written");
    let tree = repo.find_tree(tree_id).expect("fixture tree should load");
    let signature = Signature::now("Hunk Tests", "hunk@example.com")
        .expect("fixture signature should be valid");
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Initial fixture",
        &tree,
        &[],
    )
    .expect("fixture commit should be created");
}
