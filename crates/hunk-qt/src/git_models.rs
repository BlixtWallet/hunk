use std::collections::{BTreeSet, HashMap};

use hunk_git::workspace::GitWorkspaceSnapshot;
use qtbridge::{QListModel, QListModelBase, QModelItem, qobject};

#[derive(Clone, Debug, Default, QModelItem)]
pub struct GitFileItem {
    pub path: String,
    pub file_name: String,
    pub directory: String,
    pub status_tag: String,
    pub status_label: String,
    pub section: String,
    pub staged: bool,
    pub additions: i32,
    pub removals: i32,
}

#[derive(Clone, Debug, Default, QModelItem)]
pub struct GitBranchItem {
    pub name: String,
    pub current: bool,
    pub remote: bool,
    pub workspace_label: String,
}

#[derive(Clone, Debug, Default, QModelItem)]
pub struct GitCommitItem {
    pub commit_id: String,
    pub short_id: String,
    pub subject: String,
    pub committed_unix_time: i64,
}

#[derive(Clone, Debug, Default)]
pub struct GitSnapshotPayload {
    pub root: String,
    pub repository_name: String,
    pub branch_name: String,
    pub branch_has_upstream: bool,
    pub branch_ahead_count: i32,
    pub branch_behind_count: i32,
    pub changed_file_count: i32,
    pub staged_file_count: i32,
    pub unstaged_file_count: i32,
    pub last_commit_subject: String,
    pub files: Vec<GitFileItem>,
    pub branches: Vec<GitBranchItem>,
    pub commits: Vec<GitCommitItem>,
}

impl From<GitWorkspaceSnapshot> for GitSnapshotPayload {
    fn from(snapshot: GitWorkspaceSnapshot) -> Self {
        let changed_file_count = saturating_usize_to_i32(snapshot.files.len());
        let staged_file_count =
            saturating_usize_to_i32(snapshot.files.iter().filter(|file| file.staged).count());
        let unstaged_file_count = saturating_usize_to_i32(
            snapshot
                .files
                .iter()
                .filter(|file| file.unstaged || file.untracked)
                .count(),
        );
        let repository_name = snapshot
            .root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| snapshot.root.display().to_string());
        let mut files = Vec::with_capacity(snapshot.files.len());
        for file in &snapshot.files {
            let stats = snapshot
                .file_line_stats
                .get(file.path.as_str())
                .copied()
                .unwrap_or_default();
            if file.staged {
                files.push(file_item(file, stats.added, stats.removed, true));
            }
            if file.unstaged || file.untracked {
                files.push(file_item(file, stats.added, stats.removed, false));
            }
        }
        files.sort_by(|left, right| {
            left.section
                .cmp(&right.section)
                .then_with(|| left.path.cmp(&right.path))
        });

        let mut seen_branches = BTreeSet::new();
        let branches = snapshot
            .branches
            .into_iter()
            .chain(snapshot.remote_branches)
            .filter(|branch| seen_branches.insert(branch.name.clone()))
            .map(|branch| GitBranchItem {
                name: branch.name,
                current: branch.is_current,
                remote: branch.is_remote_tracking,
                workspace_label: branch.attached_workspace_target_label.unwrap_or_default(),
            })
            .collect();
        let commits = snapshot
            .recent_commits
            .into_iter()
            .map(|commit| GitCommitItem {
                short_id: commit.commit_id.chars().take(8).collect(),
                commit_id: commit.commit_id,
                subject: commit.subject,
                committed_unix_time: commit.committed_unix_time.unwrap_or_default(),
            })
            .collect();

        Self {
            root: snapshot.root.display().to_string(),
            repository_name,
            branch_name: snapshot.branch_name,
            branch_has_upstream: snapshot.branch_has_upstream,
            branch_ahead_count: saturating_usize_to_i32(snapshot.branch_ahead_count),
            branch_behind_count: saturating_usize_to_i32(snapshot.branch_behind_count),
            changed_file_count,
            staged_file_count,
            unstaged_file_count,
            last_commit_subject: snapshot.last_commit_subject.unwrap_or_default(),
            files,
            branches,
            commits,
        }
    }
}

fn file_item(
    file: &hunk_git::git::ChangedFile,
    additions: u64,
    removals: u64,
    staged: bool,
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
        status_label: status_label(status_tag.as_str()).to_owned(),
        status_tag,
        section: if staged { "STAGED" } else { "CHANGES" }.to_owned(),
        staged,
        additions: saturating_u64_to_i32(additions),
        removals: saturating_u64_to_i32(removals),
    }
}

fn status_label(tag: &str) -> &'static str {
    match tag.as_bytes() {
        b"A" => "Added",
        b"M" => "Modified",
        b"D" => "Deleted",
        b"R" => "Renamed",
        b"U" => "Untracked",
        b"T" => "Type changed",
        b"!" => "Conflict",
        _ => "Changed",
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
            use super::{QListModel, QListModelBase, $item};

            #[derive(Default)]
            pub struct $name {
                items: Vec<$item>,
                replacement: Option<Vec<$item>>,
            }

            impl $name {
                pub fn replace(&mut self, items: Vec<$item>) {
                    self.replacement = Some(items);
                    self.reset();
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
list_model!(branch_model, GitBranchListModel, GitBranchItem);
list_model!(commit_model, GitCommitListModel, GitCommitItem);
