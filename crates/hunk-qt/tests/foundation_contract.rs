use hunk_qt::Workspace;

#[test]
fn exposes_only_retained_workspaces() {
    assert_eq!(Workspace::ALL.map(Workspace::as_str), ["diff", "git", "ai"]);
    assert_eq!(Workspace::parse("diff"), Some(Workspace::Diff));
    assert_eq!(Workspace::parse("git"), Some(Workspace::Git));
    assert_eq!(Workspace::parse("ai"), Some(Workspace::Ai));
    assert_eq!(Workspace::parse("files"), None);
    assert_eq!(Workspace::parse("editor"), None);
}

#[test]
fn qml_shell_does_not_restore_the_removed_files_product() {
    let shell = include_str!("../src/qml/Hunk/Shell.qml");
    assert!(!shell.contains("workspace: \"files\""));
    assert!(!shell.contains("workspace: \"editor\""));
}
