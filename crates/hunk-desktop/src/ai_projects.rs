use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AiProjectCatalogItem {
    pub project_path: String,
    pub name: String,
}

pub fn ai_project_catalog(paths: &[PathBuf], active_root: &Path) -> Vec<AiProjectCatalogItem> {
    let mut seen = BTreeSet::new();
    let mut catalog = paths
        .iter()
        .filter(|path| seen.insert((*path).clone()))
        .map(|path| project_item(path.as_path()))
        .collect::<Vec<_>>();

    if seen.insert(active_root.to_path_buf()) {
        catalog.push(project_item(active_root));
    }

    catalog
}

pub fn ai_project_catalog_json(paths: &[PathBuf], active_root: &Path) -> String {
    serde_json::Value::Array(
        ai_project_catalog(paths, active_root)
            .into_iter()
            .map(|project| {
                serde_json::json!({
                    "project_path": project.project_path,
                    "name": project.name,
                })
            })
            .collect(),
    )
    .to_string()
}

fn project_item(path: &Path) -> AiProjectCatalogItem {
    let display_path = path.to_string_lossy().into_owned();
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| display_path.clone());
    AiProjectCatalogItem {
        project_path: display_path,
        name,
    }
}
