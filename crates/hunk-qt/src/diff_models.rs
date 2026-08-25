use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use hunk_app::diff::{
    CachedStyledSegment, DiffSegmentQuality, DiffStreamRowKind, SyntaxTokenKind,
    build_diff_row_segment_cache_from_cells, build_diff_stream_from_patch_map,
};
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
    pub left_markup: String,
    pub left_kind: String,
    pub right_line: i32,
    pub right_text: String,
    pub right_markup: String,
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
    pub(crate) search_texts: Vec<String>,
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
        let segment_quality = if projection.rows.len() > 4_000 {
            DiffSegmentQuality::SyntaxOnly
        } else {
            DiffSegmentQuality::Detailed
        };
        let rows: Vec<DiffRowItem> = projection
            .rows
            .into_iter()
            .zip(projection.row_metadata)
            .filter(|(_, metadata)| metadata.kind != DiffStreamRowKind::FileHeader)
            .map(|(row, metadata)| {
                let segments = (metadata.kind == DiffStreamRowKind::CoreCode).then(|| {
                    build_diff_row_segment_cache_from_cells(
                        Some(summary.path.as_str()),
                        row.left.text.as_str(),
                        row.left.kind,
                        row.right.text.as_str(),
                        row.right.kind,
                        segment_quality,
                    )
                });
                let mut item = DiffRowItem::from(row);
                item.stable_id = metadata.stable_id.to_string();
                if let Some(segments) = segments {
                    item.left_markup = encode_styled_segments(segments.left.as_slice());
                    item.right_markup = encode_styled_segments(segments.right.as_slice());
                }
                item
            })
            .collect();

        let search_texts = rows.iter().map(searchable_row_text).collect();
        Self {
            path: summary.path.clone(),
            status_tag: summary.status.tag().to_owned(),
            additions: saturating_u64_to_i32(summary.line_stats.added),
            removals: saturating_u64_to_i32(summary.line_stats.removed),
            rows,
            search_texts,
        }
    }

    pub fn matching_rows(&self, query: &str) -> Vec<usize> {
        matching_text_indices(self.search_texts.as_slice(), query)
    }
}

impl From<SideBySideRow> for DiffRowItem {
    fn from(row: SideBySideRow) -> Self {
        Self {
            stable_id: String::new(),
            row_kind: row_kind_label(row.kind).to_owned(),
            left_line: optional_line_to_i32(row.left.line),
            left_text: row.left.text,
            left_markup: String::new(),
            left_kind: cell_kind_label(row.left.kind).to_owned(),
            right_line: optional_line_to_i32(row.right.line),
            right_text: row.right.text,
            right_markup: String::new(),
            right_kind: cell_kind_label(row.right.kind).to_owned(),
            text: row.text,
        }
    }
}

fn encode_styled_segments(segments: &[CachedStyledSegment]) -> String {
    let mut markup = String::new();
    for segment in segments {
        let token = syntax_token_label(segment.syntax);
        markup.push_str("<font color=\"@");
        markup.push_str(token);
        markup.push_str("@\">");
        if segment.changed {
            markup.push_str("<b>");
        }
        push_html_escaped_code(&mut markup, segment.plain_text.as_str());
        if segment.changed {
            markup.push_str("</b>");
        }
        markup.push_str("</font>");
    }
    markup
}

fn syntax_token_label(kind: SyntaxTokenKind) -> &'static str {
    match kind {
        SyntaxTokenKind::Plain => "plain",
        SyntaxTokenKind::Keyword => "keyword",
        SyntaxTokenKind::String => "string",
        SyntaxTokenKind::Number => "number",
        SyntaxTokenKind::Comment => "comment",
        SyntaxTokenKind::Function => "function",
        SyntaxTokenKind::TypeName => "type",
        SyntaxTokenKind::Constant => "constant",
        SyntaxTokenKind::Variable => "variable",
        SyntaxTokenKind::Operator => "operator",
    }
}

fn push_html_escaped_code(markup: &mut String, text: &str) {
    for character in text.chars() {
        match character {
            '&' => markup.push_str("&amp;"),
            '<' => markup.push_str("&lt;"),
            '>' => markup.push_str("&gt;"),
            '"' => markup.push_str("&quot;"),
            '\'' => markup.push_str("&#39;"),
            '@' => markup.push_str("&#64;"),
            ' ' => markup.push_str("&nbsp;"),
            '\t' => markup.push_str("&nbsp;&nbsp;&nbsp;&nbsp;"),
            _ => markup.push(character),
        }
    }
}

fn matching_text_indices(search_texts: &[String], query: &str) -> Vec<usize> {
    let query = normalized_search_query(query);
    if query.is_empty() {
        return Vec::new();
    }
    search_texts
        .iter()
        .enumerate()
        .filter_map(|(index, text)| text.contains(query.as_str()).then_some(index))
        .collect()
}

fn normalized_search_query(query: &str) -> String {
    query.trim().to_lowercase()
}

fn searchable_row_text(item: &DiffRowItem) -> String {
    let mut text =
        String::with_capacity(item.left_text.len() + item.right_text.len() + item.text.len() + 2);
    text.push_str(item.left_text.as_str());
    text.push('\n');
    text.push_str(item.right_text.as_str());
    text.push('\n');
    text.push_str(item.text.as_str());
    text.to_lowercase()
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
    use super::{DiffRowItem, QListModel, QListModelBase, matching_text_indices};

    #[derive(Default)]
    pub struct DiffRowListModel {
        items: Vec<DiffRowItem>,
        search_texts: Vec<String>,
        replacement: Option<(Vec<DiffRowItem>, Vec<String>)>,
    }

    impl DiffRowListModel {
        pub fn replace(&mut self, items: Vec<DiffRowItem>, search_texts: Vec<String>) {
            debug_assert_eq!(items.len(), search_texts.len());
            self.replacement = Some((items, search_texts));
            self.reset();
        }

        pub fn matching_rows(&self, query: &str) -> Vec<usize> {
            matching_text_indices(self.search_texts.as_slice(), query)
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
            (self.items, self.search_texts) = self.replacement.take().unwrap_or_default();
        }
    }
}

pub use row_model::DiffRowListModel;
