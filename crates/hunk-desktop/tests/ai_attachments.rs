use std::fs;
use std::path::PathBuf;

use hunk_desktop::{AI_PROMPT_MAX_ATTACHMENTS, AiAttachmentDrafts, attachment_paths_from_qml_json};
use tempfile::{TempDir, tempdir_in};

fn workspace_target_tempdir() -> TempDir {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    tempdir_in(target).expect("temporary directory should be created in workspace target")
}

#[test]
fn attachment_drafts_canonicalize_filter_and_deduplicate_images() {
    let temp = workspace_target_tempdir();
    let image = temp.path().join("capture.PNG");
    let unsupported = temp.path().join("notes.txt");
    fs::write(&image, b"image").expect("image fixture should be written");
    fs::write(&unsupported, b"text").expect("text fixture should be written");

    let mut drafts = AiAttachmentDrafts::default();
    let outcome = drafts.add_paths(
        "thread",
        [
            image.clone(),
            image.clone(),
            unsupported,
            temp.path().join("missing.png"),
        ],
    );

    assert_eq!(outcome.added, 1);
    assert_eq!(outcome.skipped, 3);
    let items = drafts.items("thread");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].display_name, "capture.PNG");
    assert_eq!(drafts.paths("thread"), [fs::canonicalize(image).unwrap()]);
}

#[test]
fn attachment_drafts_enforce_the_per_prompt_bound() {
    let temp = workspace_target_tempdir();
    let mut paths = Vec::new();
    for index in 0..=AI_PROMPT_MAX_ATTACHMENTS {
        let path = temp.path().join(format!("capture-{index}.png"));
        fs::write(&path, b"image").expect("image fixture should be written");
        paths.push(path);
    }

    let mut drafts = AiAttachmentDrafts::default();
    let outcome = drafts.add_paths("thread", paths);

    assert_eq!(outcome.added, AI_PROMPT_MAX_ATTACHMENTS);
    assert_eq!(outcome.skipped, 1);
    assert_eq!(drafts.items("thread").len(), AI_PROMPT_MAX_ATTACHMENTS);
}

#[test]
fn attachment_drafts_are_isolated_and_removable_per_thread() {
    let temp = workspace_target_tempdir();
    let first = temp.path().join("first.png");
    let second = temp.path().join("second.webp");
    fs::write(&first, b"first").expect("first image fixture should be written");
    fs::write(&second, b"second").expect("second image fixture should be written");

    let mut drafts = AiAttachmentDrafts::default();
    assert_eq!(drafts.add_paths("first-thread", [first]).added, 1);
    assert_eq!(drafts.add_paths("second-thread", [second]).added, 1);

    assert_eq!(drafts.items("first-thread").len(), 1);
    assert_eq!(drafts.items("second-thread").len(), 1);
    assert!(drafts.remove("first-thread", 0));
    assert!(drafts.items("first-thread").is_empty());
    assert_eq!(drafts.items("second-thread").len(), 1);
    assert!(drafts.clear_thread("second-thread"));
    assert!(drafts.items("second-thread").is_empty());
}

#[test]
fn attachment_drafts_prune_threads_outside_the_visible_catalog() {
    let temp = workspace_target_tempdir();
    let image = temp.path().join("capture.png");
    fs::write(&image, b"image").expect("image fixture should be written");
    let mut drafts = AiAttachmentDrafts::default();
    assert_eq!(drafts.add_paths("retained", [image.clone()]).added, 1);
    assert_eq!(drafts.add_paths("pruned", [image]).added, 1);

    drafts.retain_threads(["retained"]);

    assert_eq!(drafts.items("retained").len(), 1);
    assert!(drafts.items("pruned").is_empty());
}

#[test]
fn qml_attachment_json_decodes_file_urls_and_plain_paths() {
    let json = serde_json::json!([
        "file:///Volumes/hulk/screenshot%20one.png",
        "relative/screenshot-two.jpg"
    ])
    .to_string();

    let paths = attachment_paths_from_qml_json(json.as_str()).unwrap();

    #[cfg(not(target_os = "windows"))]
    assert_eq!(
        paths[0],
        std::path::PathBuf::from("/Volumes/hulk/screenshot one.png")
    );
    assert_eq!(
        paths[1],
        std::path::PathBuf::from("relative/screenshot-two.jpg")
    );
}

#[test]
fn qml_attachment_json_rejects_invalid_or_oversized_selections() {
    let invalid_url = serde_json::json!(["file://["]).to_string();
    assert_eq!(
        attachment_paths_from_qml_json(invalid_url.as_str()).unwrap_err(),
        "The attachment selection contains an invalid local file URL."
    );

    let oversized_path = serde_json::json!(["x".repeat(32 * 1024 + 1)]).to_string();
    assert_eq!(
        attachment_paths_from_qml_json(oversized_path.as_str()).unwrap_err(),
        "The attachment selection contains a path that is too long."
    );
}
