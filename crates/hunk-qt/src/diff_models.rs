use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use hunk_app::diff::{DiffStreamRowKind, build_diff_stream_from_patch_map};
use hunk_domain::diff::{DiffCellKind, DiffRowKind, SideBySideRow};
use hunk_git::git::{ChangedFile, FileStatus, LineStats, load_patch};
use qtbridge::{QListModel, QListModelBase, QModelItem, qobject};

#[derive(Clone, Debug)]
pub struct DiffFileSummary {
    pub path: String,
    pub status: FileStatus,
    pub line_stats: LineStats,
}

#[derive(Clone, Debug, Default, QModelItem)]
pub struct DiffRowItem {
    pub stable_id: String,
    pub row_kind: String,
    pub left_line: i32,
    pub left_text: String,
    pub left_kind: String,
    pub right_line: i32,
    pub right_text: String,
    pub right_kind: String,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct DiffSnapshotPayload {
    pub path: String,
    pub status_tag: String,
    pub additions: i32,
    pub removals: i32,
    pub rows: Vec<DiffRowItem>,
}

impl DiffSnapshotPayload {
    pub fn load(root: &Path, summary: &DiffFileSummary) -> anyhow::Result<Self> {
        let patch = load_patch(root, summary.path.as_str(), summary.status)?;
        Ok(Self::from_patch(summary, patch.as_str()))
    }

    pub fn from_patch(summary: &DiffFileSummary, patch: &str) -> Self {
        let file = ChangedFile {
            path: summary.path.clone(),
            status: summary.status,
            staged: false,
            unstaged: true,
            untracked: summary.status == FileStatus::Untracked,
        };
        let files = [file];
        let line_stats = BTreeMap::from([(summary.path.clone(), summary.line_stats)]);
        let patches = BTreeMap::from([(summary.path.clone(), patch.to_owned())]);
        let projection = build_diff_stream_from_patch_map(
            &files,
            &BTreeSet::new(),
            &line_stats,
            &patches,
            &BTreeSet::new(),
        );
        let rows = projection
            .rows
            .into_iter()
            .zip(projection.row_metadata)
            .filter(|(_, metadata)| metadata.kind != DiffStreamRowKind::FileHeader)
            .map(|(row, metadata)| {
                let mut item = DiffRowItem::from(row);
                item.stable_id = metadata.stable_id.to_string();
                item
            })
            .collect();

        Self {
            path: summary.path.clone(),
            status_tag: summary.status.tag().to_owned(),
            additions: saturating_u64_to_i32(summary.line_stats.added),
            removals: saturating_u64_to_i32(summary.line_stats.removed),
            rows,
        }
    }
}

impl From<SideBySideRow> for DiffRowItem {
    fn from(row: SideBySideRow) -> Self {
        Self {
            stable_id: String::new(),
            row_kind: row_kind_label(row.kind).to_owned(),
            left_line: optional_line_to_i32(row.left.line),
            left_text: row.left.text,
            left_kind: cell_kind_label(row.left.kind).to_owned(),
            right_line: optional_line_to_i32(row.right.line),
            right_text: row.right.text,
            right_kind: cell_kind_label(row.right.kind).to_owned(),
            text: row.text,
        }
    }
}

fn row_kind_label(kind: DiffRowKind) -> &'static str {
    match kind {
        DiffRowKind::Code => "code",
        DiffRowKind::HunkHeader => "hunk",
        DiffRowKind::Meta => "meta",
        DiffRowKind::Empty => "empty",
    }
}

fn cell_kind_label(kind: DiffCellKind) -> &'static str {
    match kind {
        DiffCellKind::None => "none",
        DiffCellKind::Context => "context",
        DiffCellKind::Added => "added",
        DiffCellKind::Removed => "removed",
    }
}

fn optional_line_to_i32(line: Option<u32>) -> i32 {
    line.map_or(0, |line| i32::try_from(line).unwrap_or(i32::MAX))
}

fn saturating_u64_to_i32(value: u64) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

#[qobject(Base = QListModel)]
mod row_model {
    use super::{DiffRowItem, QListModel, QListModelBase};

    #[derive(Default)]
    pub struct DiffRowListModel {
        items: Vec<DiffRowItem>,
        replacement: Option<Vec<DiffRowItem>>,
    }

    impl DiffRowListModel {
        pub fn replace(&mut self, items: Vec<DiffRowItem>) {
            self.replacement = Some(items);
            self.reset();
        }
    }

    impl QListModel for DiffRowListModel {
        type Item = DiffRowItem;

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

pub use row_model::DiffRowListModel;
