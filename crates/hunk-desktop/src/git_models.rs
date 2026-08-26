use std::collections::HashMap;

use hunk_git::git::{ChangedFile, LineStats};
use hunk_git::workspace::GitWorkspaceSnapshot;
use qtbridge::{QListModel, QListModelBase, QModelItem, qobject};

use crate::DiffCompareSourceCatalog;
use crate::diff_models::DiffFileSummary;

#[derive(Clone, Debug, Default, QModelItem)]
pub struct GitFileItem {
    pub path: String,
    pub file_name: String,
    pub directory: String,
    pub status_tag: String,
    pub additions: i32,
    pub removals: i32,
}

#[derive(Clone, Debug, Default)]
pub struct GitSnapshotPayload {
    pub root: String,
    pub repository_name: String,
    pub branch_name: String,
    pub changed_file_count: i32,
    pub diff_files: Vec<GitFileItem>,
    pub diff_file_summaries: Vec<DiffFileSummary>,
    pub compare_catalog: DiffCompareSourceCatalog,
}

impl From<GitWorkspaceSnapshot> for GitSnapshotPayload {
    fn from(snapshot: GitWorkspaceSnapshot) -> Self {
        let changed_file_count = saturating_usize_to_i32(snapshot.files.len());
        let compare_catalog =
            DiffCompareSourceCatalog::load(snapshot.root.as_path(), snapshot.branches.as_slice())
                .unwrap_or_default();
        let repository_name = snapshot
            .root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| snapshot.root.display().to_string());
        let mut diff_files = snapshot
            .files
            .iter()
            .map(|file| {
                let stats = file_line_stats(&snapshot, file);
                file_item(file, stats.added, stats.removed)
            })
            .collect::<Vec<_>>();
        diff_files.sort_by(|left, right| left.path.cmp(&right.path));
        let diff_file_summaries = snapshot
            .files
            .iter()
            .map(|file| DiffFileSummary {
                path: file.path.clone(),
                status: file.status,
                line_stats: file_line_stats(&snapshot, file),
            })
            .collect();
        Self {
            root: snapshot.root.display().to_string(),
            repository_name,
            branch_name: snapshot.branch_name,
            changed_file_count,
            diff_files,
            diff_file_summaries,
            compare_catalog,
        }
    }
}

fn file_line_stats(snapshot: &GitWorkspaceSnapshot, file: &ChangedFile) -> LineStats {
    snapshot
        .file_line_stats
        .get(file.path.as_str())
        .copied()
        .unwrap_or_default()
}

pub(crate) fn file_item(
    file: &hunk_git::git::ChangedFile,
    additions: u64,
    removals: u64,
) -> GitFileItem {
    let (directory, file_name) = file.path.rsplit_once('/').map_or_else(
        || (String::new(), file.path.clone()),
        |(directory, name)| (directory.to_owned(), name.to_owned()),
    );
    let status_tag = file.status.tag().to_owned();

    GitFileItem {
        path: file.path.clone(),
        file_name,
        directory,
        status_tag,
        additions: saturating_u64_to_i32(additions),
        removals: saturating_u64_to_i32(removals),
    }
}

fn saturating_usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn saturating_u64_to_i32(value: u64) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

macro_rules! list_model {
    ($module:ident, $name:ident, $item:ident) => {
        #[qobject(Base = QListModel)]
        mod $module {
            use qtbridge::QObjectHolder;

            use super::{QListModel, QListModelBase, $item};

            #[derive(Default)]
            pub struct $name {
                items: Vec<$item>,
                replacement: Option<Vec<$item>>,
                deferred_replacement: Option<Vec<$item>>,
                deferred_update_scheduled: bool,
            }

            impl $name {
                pub fn replace(&mut self, items: Vec<$item>) {
                    self.replacement = Some(items);
                    self.reset();
                }

                pub fn defer_replace(&mut self, items: Vec<$item>) {
                    self.deferred_replacement = Some(items);
                    if self.deferred_update_scheduled {
                        return;
                    }
                    self.deferred_update_scheduled = true;
                    if !self
                        .get_qml_method_invoker()
                        .invoke_method("apply_deferred_replacement")
                    {
                        self.deferred_update_scheduled = false;
                    }
                }

                #[qslot]
                fn apply_deferred_replacement(&mut self) {
                    self.deferred_update_scheduled = false;
                    let Some(items) = self.deferred_replacement.take() else {
                        return;
                    };
                    self.replace(items);
                }
            }

            impl QListModel for $name {
                type Item = $item;

                fn len(&self) -> usize {
                    self.items.len()
                }

                fn get(&self, index: usize) -> Option<&Self::Item> {
                    self.items.get(index)
                }

                fn reset_unnotified(&mut self) {
                    self.items = self.replacement.take().unwrap_or_default();
                }
            }
        }

        pub use $module::$name;
    };
}

list_model!(file_model, GitFileListModel, GitFileItem);
