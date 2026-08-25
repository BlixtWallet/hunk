use hunk_desktop::{Workspace, local_path_from_qml_folder_url};

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

#[test]
fn preserves_plain_repository_folder_paths() {
    let path = local_path_from_qml_folder_url("relative/repository")
        .expect("plain paths should remain valid");
    assert_eq!(path, std::path::PathBuf::from("relative/repository"));
}

#[cfg(not(target_os = "windows"))]
#[test]
fn decodes_local_qml_folder_urls() {
    let path = local_path_from_qml_folder_url("file:///Volumes/hulk/project%20with%20spaces")
        .expect("local file URLs should decode");
    assert_eq!(
        path,
        std::path::PathBuf::from("/Volumes/hulk/project with spaces")
    );
}

#[cfg(target_os = "windows")]
#[test]
fn decodes_local_qml_folder_urls() {
    let path = local_path_from_qml_folder_url("file:///C:/Hunk/project%20with%20spaces")
        .expect("local file URLs should decode");
    assert_eq!(
        path,
        std::path::PathBuf::from(r"C:\Hunk\project with spaces")
    );
}
