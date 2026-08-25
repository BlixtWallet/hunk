use std::fs;
use std::path::Path;

use git2::{Repository, Signature};
use hunk_git::workspace::{GitWorkspaceCommand, execute_git_workspace_command, load_git_workspace};

#[test]
fn workspace_commands_cover_index_commit_restore_and_branch_flows() {
    let temp = tempfile::tempdir().expect("temporary repository should be created");
    initialize_repository(temp.path());
    fs::write(temp.path().join("README.md"), "# changed fixture\n")
        .expect("tracked file should be modified");

    let initial = load_git_workspace(temp.path()).expect("Git workspace should load");
    assert_eq!(initial.branch_name, "master");
    assert!(
        initial
            .files
            .iter()
            .any(|file| file.path == "README.md" && file.unstaged)
    );
    assert!(!initial.recent_commits.is_empty());

    execute_git_workspace_command(
        temp.path(),
        GitWorkspaceCommand::StagePaths(vec!["README.md".to_owned()]),
    )
    .expect("tracked change should stage");
    let staged = load_git_workspace(temp.path()).expect("staged workspace should load");
    assert!(
        staged
            .files
            .iter()
            .any(|file| file.path == "README.md" && file.staged)
    );

    execute_git_workspace_command(
        temp.path(),
        GitWorkspaceCommand::UnstagePaths(vec!["README.md".to_owned()]),
    )
    .expect("tracked change should unstage");
    let unstaged = load_git_workspace(temp.path()).expect("unstaged workspace should load");
    assert!(
        unstaged
            .files
            .iter()
            .any(|file| file.path == "README.md" && file.unstaged)
    );

    execute_git_workspace_command(
        temp.path(),
        GitWorkspaceCommand::StagePaths(vec!["README.md".to_owned()]),
    )
    .expect("tracked change should restage");
    execute_git_workspace_command(
        temp.path(),
        GitWorkspaceCommand::CommitStaged {
            message: "Update fixture".to_owned(),
        },
    )
    .expect("staged change should commit");

    let committed = load_git_workspace(temp.path()).expect("committed workspace should load");
    assert!(committed.files.is_empty());
    assert_eq!(
        committed.last_commit_subject.as_deref(),
        Some("Update fixture")
    );

    fs::write(temp.path().join("README.md"), "discard me\n")
        .expect("tracked file should be modified again");
    execute_git_workspace_command(
        temp.path(),
        GitWorkspaceCommand::RestorePaths(vec!["README.md".to_owned()]),
    )
    .expect("tracked change should restore");
    assert_eq!(
        fs::read_to_string(temp.path().join("README.md"))
            .expect("restored file should be readable"),
        "# changed fixture\n"
    );

    execute_git_workspace_command(
        temp.path(),
        GitWorkspaceCommand::ActivateBranch {
            name: "review/qt-git".to_owned(),
        },
    )
    .expect("clean workspace should activate a new branch");
    assert_eq!(
        load_git_workspace(temp.path())
            .expect("branch workspace should load")
            .branch_name,
        "review/qt-git"
    );
}

fn initialize_repository(root: &Path) {
    let repo = Repository::init(root).expect("repository should initialize");
    let mut config = repo.config().expect("repository config should open");
    config
        .set_str("user.name", "Hunk Tests")
        .expect("test user name should be configured");
    config
        .set_str("user.email", "hunk@example.com")
        .expect("test user email should be configured");
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
