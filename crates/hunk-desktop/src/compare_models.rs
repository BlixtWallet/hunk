use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use hunk_git::compare::{
    CompareSnapshot, CompareSource, compare_branch_source_id, compare_workspace_target_source_id,
};
use hunk_git::git::LocalBranch;
use hunk_git::worktree::{WorkspaceTargetKind, list_workspace_targets};
use qtbridge::{QListModel, QListModelBase, QModelItem, qobject};

use crate::diff_models::DiffFileSummary;
use crate::git_models::{GitFileItem, file_item};

#[derive(Clone, Debug, Default, Eq, PartialEq, QModelItem)]
pub struct DiffCompareSourceItem {
    pub source_id: String,
    pub label: String,
    pub detail: String,
    pub kind: String,
    pub branch_name: String,
    pub target_id: String,
    pub root: String,
}

impl DiffCompareSourceItem {
    pub fn compare_source(&self) -> Option<CompareSource> {
        match self.kind.as_str() {
            "branch" => Some(CompareSource::Branch {
                name: self.branch_name.clone(),
            }),
            "workspace" => Some(CompareSource::WorkspaceTarget {
                target_id: self.target_id.clone(),
                root: self.root.clone().into(),
            }),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DiffCompareSourceCatalog {
    pub items: Vec<DiffCompareSourceItem>,
    pub default_left_source_id: String,
    pub default_right_source_id: String,
}

impl DiffCompareSourceCatalog {
    pub fn load(repo_root: &Path, branches: &[LocalBranch]) -> anyhow::Result<Self> {
        let targets = list_workspace_targets(repo_root)?;
        let mut seen_ids = BTreeSet::new();
        let mut items = Vec::with_capacity(targets.len().saturating_add(branches.len()));

        for target in &targets {
            let source_id = compare_workspace_target_source_id(target.id.as_str());
            if !seen_ids.insert(source_id.clone()) {
                continue;
            }
            let detail = match target.kind {
                WorkspaceTargetKind::PrimaryCheckout => {
                    format!("Primary checkout · {}", target.branch_name)
                }
                WorkspaceTargetKind::LinkedWorktree if target.managed => {
                    format!("Managed worktree · {}", target.branch_name)
                }
                WorkspaceTargetKind::LinkedWorktree => {
                    format!("Linked worktree · {}", target.branch_name)
                }
            };
            items.push(DiffCompareSourceItem {
                source_id,
                label: target.display_name.clone(),
                detail,
                kind: "workspace".to_owned(),
                branch_name: target.branch_name.clone(),
                target_id: target.id.clone(),
                root: target.root.display().to_string(),
            });
        }

        for branch in branches {
            let source_id = compare_branch_source_id(branch.name.as_str());
            if !seen_ids.insert(source_id.clone()) {
                continue;
            }
            items.push(DiffCompareSourceItem {
                source_id,
                label: branch.name.clone(),
                detail: if branch.is_current {
                    "Local branch · checked out".to_owned()
                } else {
                    "Local branch".to_owned()
                },
                kind: "branch".to_owned(),
                branch_name: branch.name.clone(),
                target_id: String::new(),
                root: String::new(),
            });
        }

        let right = targets
            .iter()
            .find(|target| target.is_active)
            .or_else(|| targets.first())
            .map(|target| compare_workspace_target_source_id(target.id.as_str()));
        let right_branch = targets
            .iter()
            .find(|target| target.is_active)
            .or_else(|| targets.first())
            .map(|target| target.branch_name.as_str());
        let left = right_branch
            .filter(|branch| !matches!(*branch, "detached" | "unborn"))
            .map(compare_branch_source_id)
            .filter(|source_id| items.iter().any(|item| item.source_id == *source_id))
            .or_else(|| {
                items
                    .iter()
                    .find(|item| Some(item.source_id.as_str()) != right.as_deref())
                    .map(|item| item.source_id.clone())
            });

        Ok(Self {
            items,
            default_left_source_id: left.unwrap_or_default(),
            default_right_source_id: right.unwrap_or_default(),
        })
    }
}

pub struct DiffCompareSnapshotPayload {
    pub files: Vec<GitFileItem>,
    pub file_summaries: Vec<DiffFileSummary>,
    pub patches_by_path: HashMap<String, String>,
}

impl From<CompareSnapshot> for DiffCompareSnapshotPayload {
    fn from(snapshot: CompareSnapshot) -> Self {
        let mut files = snapshot
            .files
            .iter()
            .map(|file| {
                let stats = snapshot
                    .file_line_stats
                    .get(file.path.as_str())
                    .copied()
                    .unwrap_or_default();
                file_item(file, stats.added, stats.removed)
            })
            .collect::<Vec<_>>();
        files.sort_by(|left, right| left.path.cmp(&right.path));
        let file_summaries = snapshot
            .files
            .into_iter()
            .map(|file| DiffFileSummary {
                line_stats: snapshot
                    .file_line_stats
                    .get(file.path.as_str())
                    .copied()
                    .unwrap_or_default(),
                path: file.path,
                status: file.status,
            })
            .collect();

        Self {
            files,
            file_summaries,
            patches_by_path: snapshot.patches_by_path.into_iter().collect(),
        }
    }
}

#[qobject(Base = QListModel)]
mod source_model {
    use qtbridge::QObjectHolder;

    use super::{DiffCompareSourceItem, QListModel, QListModelBase};

    #[derive(Default)]
    pub struct DiffCompareSourceListModel {
        items: Vec<DiffCompareSourceItem>,
        replacement: Option<Vec<DiffCompareSourceItem>>,
        deferred_replacement: Option<Vec<DiffCompareSourceItem>>,
        deferred_update_scheduled: bool,
    }

    impl DiffCompareSourceListModel {
        pub fn replace(&mut self, items: Vec<DiffCompareSourceItem>) {
            self.replacement = Some(items);
            self.reset();
        }

        pub fn defer_replace(&mut self, items: Vec<DiffCompareSourceItem>) {
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

        pub fn index_of(&self, source_id: &str) -> Option<usize> {
            self.visible_items()
                .iter()
                .position(|item| item.source_id == source_id)
        }

        pub fn item_at(&self, index: i32) -> Option<DiffCompareSourceItem> {
            usize::try_from(index)
                .ok()
                .and_then(|index| self.visible_items().get(index))
                .cloned()
        }

        pub fn contains(&self, source_id: &str) -> bool {
            self.index_of(source_id).is_some()
        }

        pub fn first_except(&self, source_id: &str) -> Option<DiffCompareSourceItem> {
            self.visible_items()
                .iter()
                .find(|item| item.source_id != source_id)
                .cloned()
        }

        fn visible_items(&self) -> &[DiffCompareSourceItem] {
            self.deferred_replacement
                .as_deref()
                .unwrap_or(self.items.as_slice())
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

    impl QListModel for DiffCompareSourceListModel {
        type Item = DiffCompareSourceItem;

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

pub use source_model::DiffCompareSourceListModel;
