use std::path::{Path, PathBuf};

use hunk_desktop::{AiProjectCatalogItem, ai_project_catalog, ai_project_catalog_json};

#[test]
fn project_catalog_preserves_saved_order_and_removes_duplicates() {
    let paths = vec![
        PathBuf::from("/tmp/glab"),
        PathBuf::from("/tmp/lightning-service-rust"),
        PathBuf::from("/tmp/glab"),
    ];

    assert_eq!(
        ai_project_catalog(&paths, Path::new("/tmp/lightning-service-rust")),
        vec![
            AiProjectCatalogItem {
                project_path: "/tmp/glab".to_owned(),
                name: "glab".to_owned(),
            },
            AiProjectCatalogItem {
                project_path: "/tmp/lightning-service-rust".to_owned(),
                name: "lightning-service-rust".to_owned(),
            },
        ]
    );
}

#[test]
fn project_catalog_includes_the_active_root_when_it_is_not_saved_yet() {
    let catalog = ai_project_catalog(&[], Path::new("/tmp/new-project"));

    assert_eq!(
        catalog,
        vec![AiProjectCatalogItem {
            project_path: "/tmp/new-project".to_owned(),
            name: "new-project".to_owned(),
        }]
    );
}

#[test]
fn project_catalog_json_is_valid_qml_input() {
    let json = ai_project_catalog_json(&[PathBuf::from("/tmp/glab")], Path::new("/tmp/glab"));
    let catalog = serde_json::from_str::<serde_json::Value>(&json).unwrap();

    assert_eq!(catalog.as_array().unwrap().len(), 1);
    assert_eq!(catalog[0]["name"], "glab");
}
