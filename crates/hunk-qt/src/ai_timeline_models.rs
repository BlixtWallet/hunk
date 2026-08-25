use std::collections::{BTreeSet, HashMap};

use hunk_codex::state::{AiState, ItemStatus, ItemSummary, TurnPlanStepStatus, TurnPlanSummary};
use qtbridge::{QListModel, QListModelBase, QModelItem, qobject};

const AI_TIMELINE_MAX_VISIBLE_TURNS: usize = 80;
const AI_TIMELINE_MAX_VISIBLE_ROWS: usize = 1_000;
const AI_TIMELINE_MAX_TEXT_BYTES: usize = 16 * 1024;
const AI_TIMELINE_MAX_TITLE_BYTES: usize = 240;

#[derive(Clone, Debug, Default, Eq, PartialEq, QModelItem)]
pub struct AiTimelineItem {
    pub row_id: String,
    pub turn_id: String,
    pub kind: String,
    pub role: String,
    pub title: String,
    pub text: String,
    pub status: String,
    pub streaming: bool,
    pub mono: bool,
    pub truncated: bool,
    pub last_sequence: i64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AiTimelineProjection {
    pub items: Vec<AiTimelineItem>,
    pub total_turn_count: i32,
    pub visible_turn_count: i32,
    pub hidden_turn_count: i32,
    pub total_row_count: i32,
    pub hidden_row_count: i32,
}

enum TimelineSource<'a> {
    Item(&'a str, &'a ItemSummary),
    Plan(&'a str, &'a TurnPlanSummary),
}

impl TimelineSource<'_> {
    fn sequence(&self) -> u64 {
        match self {
            Self::Item(_, item) => item.last_sequence,
            Self::Plan(_, plan) => plan.last_sequence,
        }
    }

    fn sort_key(&self) -> (u8, &str) {
        match self {
            Self::Item(key, _) => (0, key),
            Self::Plan(key, _) => (1, key),
        }
    }

    fn project(self) -> AiTimelineItem {
        match self {
            Self::Item(key, item) => timeline_item(key, item),
            Self::Plan(key, plan) => timeline_plan(key, plan),
        }
    }
}

impl AiTimelineProjection {
    pub fn from_state(state: &AiState, thread_id: Option<&str>) -> Self {
        let Some(thread_id) = thread_id.filter(|thread_id| !thread_id.is_empty()) else {
            return Self::default();
        };

        let mut turns = state
            .turns
            .values()
            .filter(|turn| turn.thread_id == thread_id)
            .collect::<Vec<_>>();
        turns.sort_by(|left, right| {
            left.last_sequence
                .cmp(&right.last_sequence)
                .then_with(|| left.id.cmp(&right.id))
        });
        let total_turn_count = turns.len();
        let hidden_turn_count = total_turn_count.saturating_sub(AI_TIMELINE_MAX_VISIBLE_TURNS);
        let visible_turn_ids = turns
            .iter()
            .skip(hidden_turn_count)
            .map(|turn| turn.id.as_str())
            .collect::<BTreeSet<_>>();

        let mut rows = state
            .items
            .iter()
            .filter(|(_, item)| {
                item.thread_id == thread_id
                    && visible_turn_ids.contains(item.turn_id.as_str())
                    && item_is_renderable(item)
            })
            .map(|(item_key, item)| TimelineSource::Item(item_key, item))
            .collect::<Vec<_>>();
        rows.extend(
            state
                .turn_plans
                .iter()
                .filter(|(_, plan)| {
                    plan.thread_id == thread_id
                        && visible_turn_ids.contains(plan.turn_id.as_str())
                        && (plan
                            .explanation
                            .as_deref()
                            .is_some_and(|value| !value.trim().is_empty())
                            || !plan.steps.is_empty())
                })
                .map(|(turn_key, plan)| TimelineSource::Plan(turn_key, plan)),
        );
        rows.sort_by(|left, right| {
            left.sequence()
                .cmp(&right.sequence())
                .then_with(|| left.sort_key().cmp(&right.sort_key()))
        });

        let total_row_count = rows.len();
        let hidden_row_count = total_row_count.saturating_sub(AI_TIMELINE_MAX_VISIBLE_ROWS);
        let items = rows
            .into_iter()
            .skip(hidden_row_count)
            .map(TimelineSource::project)
            .collect();

        Self {
            items,
            total_turn_count: saturating_usize_to_i32(total_turn_count),
            visible_turn_count: saturating_usize_to_i32(
                total_turn_count.saturating_sub(hidden_turn_count),
            ),
            hidden_turn_count: saturating_usize_to_i32(hidden_turn_count),
            total_row_count: saturating_usize_to_i32(total_row_count),
            hidden_row_count: saturating_usize_to_i32(hidden_row_count),
        }
    }
}

fn timeline_item(item_key: &str, item: &hunk_codex::state::ItemSummary) -> AiTimelineItem {
    let content = item.content.trim();
    let text_source = if content.is_empty() {
        item.display_metadata
            .as_ref()
            .and_then(|metadata| metadata.details_json.as_deref())
            .map(str::trim)
            .unwrap_or_default()
    } else {
        content
    };
    let (text, truncated) = bounded_text(text_source, AI_TIMELINE_MAX_TEXT_BYTES);
    AiTimelineItem {
        row_id: format!("item:{item_key}"),
        turn_id: item.turn_id.clone(),
        kind: item.kind.clone(),
        role: item_role(item.kind.as_str()).to_owned(),
        title: bounded_text(item_title(item).as_str(), AI_TIMELINE_MAX_TITLE_BYTES).0,
        text,
        status: item_status_label(item.status).to_owned(),
        streaming: item.status != ItemStatus::Completed,
        mono: item_uses_mono_text(item.kind.as_str()),
        truncated,
        last_sequence: saturating_u64_to_i64(item.last_sequence),
    }
}

fn timeline_plan(turn_key: &str, plan: &hunk_codex::state::TurnPlanSummary) -> AiTimelineItem {
    let mut text = plan
        .explanation
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
        .to_owned();
    for step in &plan.steps {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(match step.status {
            TurnPlanStepStatus::Pending => "[ ] ",
            TurnPlanStepStatus::InProgress => "[~] ",
            TurnPlanStepStatus::Completed => "[x] ",
        });
        text.push_str(step.step.trim());
    }
    let (text, truncated) = bounded_text(text.trim(), AI_TIMELINE_MAX_TEXT_BYTES);
    let streaming = plan
        .steps
        .iter()
        .any(|step| step.status == TurnPlanStepStatus::InProgress);

    AiTimelineItem {
        row_id: format!("turn-plan:{turn_key}"),
        turn_id: plan.turn_id.clone(),
        kind: "turnPlan".to_owned(),
        role: "assistant".to_owned(),
        title: "Plan".to_owned(),
        text,
        status: if streaming { "in progress" } else { "" }.to_owned(),
        streaming,
        mono: false,
        truncated,
        last_sequence: saturating_u64_to_i64(plan.last_sequence),
    }
}

fn item_is_renderable(item: &hunk_codex::state::ItemSummary) -> bool {
    if !matches!(item.kind.as_str(), "reasoning" | "webSearch") {
        return true;
    }
    !item.content.trim().is_empty()
        || item.display_metadata.as_ref().is_some_and(|metadata| {
            metadata
                .summary
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
                || metadata
                    .details_json
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
        })
}

fn item_title(item: &hunk_codex::state::ItemSummary) -> String {
    if let Some(summary) = item
        .display_metadata
        .as_ref()
        .and_then(|metadata| metadata.summary.as_deref())
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
    {
        return summary.to_owned();
    }
    match item.kind.as_str() {
        "userMessage" => "You",
        "agentMessage" => "Assistant",
        "plan" => "Proposed plan",
        "reasoning" => "Reasoning",
        "commandExecution" => "Command",
        "fileChange" => "File change",
        "mcpToolCall" => "MCP tool",
        "dynamicToolCall" => "Tool",
        "collabAgentToolCall" => "Collaboration",
        "webSearch" => "Web search",
        "imageView" => "Image",
        "enteredReviewMode" => "Review mode entered",
        "exitedReviewMode" => "Review mode exited",
        "contextCompaction" => "Context compacted",
        kind => kind,
    }
    .to_owned()
}

fn item_role(kind: &str) -> &'static str {
    match kind {
        "userMessage" => "user",
        "agentMessage" | "plan" | "reasoning" | "webSearch" => "assistant",
        "commandExecution"
        | "fileChange"
        | "mcpToolCall"
        | "dynamicToolCall"
        | "collabAgentToolCall" => "tool",
        _ => "system",
    }
}

fn item_uses_mono_text(kind: &str) -> bool {
    matches!(
        kind,
        "commandExecution"
            | "fileChange"
            | "mcpToolCall"
            | "dynamicToolCall"
            | "collabAgentToolCall"
    )
}

fn item_status_label(status: ItemStatus) -> &'static str {
    match status {
        ItemStatus::Started => "started",
        ItemStatus::Streaming => "streaming",
        ItemStatus::Completed => "",
    }
}

fn bounded_text(text: &str, max_bytes: usize) -> (String, bool) {
    if text.len() <= max_bytes {
        return (text.to_owned(), false);
    }
    const ELLIPSIS: &str = "…";
    let mut end = max_bytes.saturating_sub(ELLIPSIS.len()).min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut bounded = text[..end].trim_end().to_owned();
    bounded.push_str(ELLIPSIS);
    (bounded, true)
}

fn saturating_usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn saturating_u64_to_i64(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[qobject(Base = QListModel)]
mod timeline_model {
    use super::{AiTimelineItem, QListModel, QListModelBase};

    #[derive(Default)]
    pub struct AiTimelineListModel {
        items: Vec<AiTimelineItem>,
        replacement: Option<Vec<AiTimelineItem>>,
    }

    impl AiTimelineListModel {
        pub fn sync(&mut self, items: Vec<AiTimelineItem>) -> bool {
            let stable_rows = self.items.len() == items.len()
                && self
                    .items
                    .iter()
                    .zip(&items)
                    .all(|(current, next)| current.row_id == next.row_id);
            if stable_rows {
                let mut changed = false;
                for (index, item) in items.into_iter().enumerate() {
                    if self.items[index] != item {
                        let _ = self.set(index, item);
                        changed = true;
                    }
                }
                return changed;
            } else {
                self.replacement = Some(items);
                self.reset();
            }
            true
        }

        pub fn replace(&mut self, items: Vec<AiTimelineItem>) {
            self.replacement = Some(items);
            self.reset();
        }
    }

    impl QListModel for AiTimelineListModel {
        type Item = AiTimelineItem;

        fn len(&self) -> usize {
            self.items.len()
        }

        fn get(&self, index: usize) -> Option<&Self::Item> {
            self.items.get(index)
        }

        fn set_unnotified(&mut self, index: usize, value: Self::Item) -> bool {
            let Some(item) = self.items.get_mut(index) else {
                return false;
            };
            *item = value;
            true
        }

        fn reset_unnotified(&mut self) {
            self.items = self.replacement.take().unwrap_or_default();
        }
    }
}

pub use timeline_model::AiTimelineListModel;
