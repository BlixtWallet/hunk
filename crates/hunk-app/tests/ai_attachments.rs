use std::path::Path;

use hunk_app::ai::is_supported_ai_image_path;

#[test]
fn recognizes_codex_image_attachment_extensions_case_insensitively() {
    for path in [
        "capture.png",
        "photo.JPG",
        "photo.jpeg",
        "preview.webp",
        "scan.bmp",
        "animation.gif",
        "document.tif",
        "document.tiff",
    ] {
        assert!(is_supported_ai_image_path(Path::new(path)), "{path}");
    }
}

#[test]
fn rejects_non_image_and_extensionless_paths() {
    for path in ["notes.txt", "vector.svg", "archive.zip", "screenshot"] {
        assert!(!is_supported_ai_image_path(Path::new(path)), "{path}");
    }
}
