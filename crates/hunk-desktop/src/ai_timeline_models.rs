use std::collections::{BTreeSet, HashMap, HashSet};

use hunk_codex::state::{AiState, ItemStatus, ItemSummary, TurnPlanStepStatus, TurnPlanSummary};
use qtbridge::{QListModel, QListModelBase, QModelItem, qobject};

use crate::ai_markdown::{
    AI_MARKDOWN_MAX_VISIBLE_MESSAGES, AiMarkdownBlockProjection, AiMarkdownProjectionCache,
};

const AI_TIMELINE_MAX_VISIBLE_TURNS: usize = 80;
pub const AI_TIMELINE_MAX_VISIBLE_ROWS: usize = 1_000;
const AI_TIMELINE_MAX_TEXT_BYTES: usize = 16 * 1024;
const AI_TIMELINE_MAX_TITLE_BYTES: usize = 240;
const AI_COMMAND_DISPLAY_BYTES: usize = 2 * 1024;
const AI_COMMAND_CWD_DISPLAY_BYTES: usize = 1024;

#[derive(Clone, Debug, Default, Eq, PartialEq, QModelItem)]
pub struct AiTimelineItem {
    pub row_id: String,
    pub turn_id: String,
    pub kind: String,
    pub role: String,
    pub title: String,
    pub text: String,
    pub markdown_kind: String,
    pub markdown_markup: String,
    pub markdown_language: String,
    pub markdown_heading_level: i32,
    pub markdown_first: bool,
    pub markdown_last: bool,
    pub command: String,
    pub cwd: String,
    pub status: String,
    pub streaming: bool,
    pub mono: bool,
    pub truncated: bool,
    pub last_sequence: i64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AiTimelineProjection {
    pub items: Vec<AiTimelineItem>,
    pub active_turn_id: String,
    pub turn_running: bool,
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
        let mut markdown_cache = AiMarkdownProjectionCache::default();
        Self::from_state_with_markdown_cache(state, thread_id, &mut markdown_cache)
    }

    pub(crate) fn from_state_with_markdown_cache(
        state: &AiState,
        thread_id: Option<&str>,
        markdown_cache: &mut AiMarkdownProjectionCache,
    ) -> Self {
        let Some(thread_id) = thread_id.filter(|thread_id| !thread_id.is_empty()) else {
            markdown_cache.clear();
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
        let active_turn_id = turns
            .iter()
            .rev()
            .find(|turn| turn.status == hunk_codex::state::TurnStatus::InProgress)
            .map(|turn| turn.id.clone())
            .unwrap_or_default();
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

        let source_total_row_count = rows.len();
        let source_hidden_row_count =
            source_total_row_count.saturating_sub(AI_TIMELINE_MAX_VISIBLE_ROWS);
        let source_items = rows
            .into_iter()
            .skip(source_hidden_row_count)
            .map(TimelineSource::project)
            .collect::<Vec<_>>();
        let markdown_row_ids = source_items
            .iter()
            .rev()
            .filter(|item| item.kind == "agentMessage" && !item.streaming)
            .take(AI_MARKDOWN_MAX_VISIBLE_MESSAGES)
            .map(|item| item.row_id.clone())
            .collect::<HashSet<_>>();
        let mut projected_items = Vec::with_capacity(source_items.len());
        for item in source_items {
            if markdown_row_ids.contains(item.row_id.as_str()) {
                let blocks = markdown_cache.project_completed_message(
                    item.row_id.as_str(),
                    item.last_sequence,
                    item.text.as_str(),
                );
                if let Some(blocks) = blocks {
                    projected_items.extend(expand_markdown_item(item, blocks));
                } else {
                    projected_items.push(item);
                }
            } else {
                markdown_cache.remove(item.row_id.as_str());
                projected_items.push(item);
            }
        }
        markdown_cache.retain_visible(markdown_row_ids.iter().map(String::as_str));

        let projected_hidden_row_count = projected_items
            .len()
            .saturating_sub(AI_TIMELINE_MAX_VISIBLE_ROWS);
        let mut items = projected_items
            .into_iter()
            .skip(projected_hidden_row_count)
            .collect::<Vec<_>>();
        if let Some(first) = items.first_mut()
            && !first.markdown_kind.is_empty()
        {
            first.markdown_first = true;
        }
        let hidden_row_count = source_hidden_row_count.saturating_add(projected_hidden_row_count);
        let total_row_count = hidden_row_count.saturating_add(items.len());

        Self {
            items,
            turn_running: !active_turn_id.is_empty(),
            active_turn_id,
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

fn expand_markdown_item(
    item: AiTimelineItem,
    blocks: Vec<AiMarkdownBlockProjection>,
) -> Vec<AiTimelineItem> {
    let block_count = blocks.len();
    blocks
        .into_iter()
        .enumerate()
        .map(|(index, block)| {
            let first = index == 0;
            let last = index + 1 == block_count;
            AiTimelineItem {
                row_id: format!("{}:markdown:{index}", item.row_id),
                turn_id: item.turn_id.clone(),
                kind: item.kind.clone(),
                role: item.role.clone(),
                title: item.title.clone(),
                text: block.text,
                markdown_kind: block.kind,
                markdown_markup: block.markup,
                markdown_language: block.language,
                markdown_heading_level: block.heading_level,
                markdown_first: first,
                markdown_last: last,
                command: String::new(),
                cwd: String::new(),
                status: String::new(),
                streaming: false,
                mono: false,
                truncated: item.truncated && last,
                last_sequence: item.last_sequence,
            }
        })
        .collect()
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
    let (command, cwd) = command_execution_target(item);
    AiTimelineItem {
        row_id: format!("item:{item_key}"),
        turn_id: item.turn_id.clone(),
        kind: item.kind.clone(),
        role: item_role(item.kind.as_str()).to_owned(),
        title: bounded_text(item_title(item).as_str(), AI_TIMELINE_MAX_TITLE_BYTES).0,
        text,
        markdown_kind: String::new(),
        markdown_markup: String::new(),
        markdown_language: String::new(),
        markdown_heading_level: 0,
        markdown_first: false,
        markdown_last: false,
        command,
        cwd,
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
        markdown_kind: String::new(),
        markdown_markup: String::new(),
        markdown_language: String::new(),
        markdown_heading_level: 0,
        markdown_first: false,
        markdown_last: false,
        command: String::new(),
        cwd: String::new(),
        status: if streaming { "in progress" } else { "" }.to_owned(),
        streaming,
        mono: false,
        truncated,
        last_sequence: saturating_u64_to_i64(plan.last_sequence),
    }
}

fn command_execution_target(item: &hunk_codex::state::ItemSummary) -> (String, String) {
    if item.kind != "commandExecution" {
        return (String::new(), String::new());
    }
    let Some(details) = item
        .display_metadata
        .as_ref()
        .and_then(|metadata| metadata.details_json.as_deref())
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
    else {
        return (String::new(), String::new());
    };
    if details.get("kind").and_then(serde_json::Value::as_str) != Some("commandExecution") {
        return (String::new(), String::new());
    }
    let command = details
        .get("command")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    let cwd = details
        .get("cwd")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    if command.len() > AI_COMMAND_DISPLAY_BYTES || cwd.len() > AI_COMMAND_CWD_DISPLAY_BYTES {
        return (String::new(), String::new());
    }
    (command, cwd)
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
    use qtbridge::QObjectHolder;

    use super::{AI_TIMELINE_MAX_VISIBLE_ROWS, AiTimelineItem, QListModel, QListModelBase};

    #[derive(Default)]
    pub struct AiTimelineListModel {
        items: Vec<AiTimelineItem>,
        replacement: Option<Vec<AiTimelineItem>>,
        deferred_items: Option<Vec<AiTimelineItem>>,
        deferred_update_scheduled: bool,
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

        pub fn defer_sync(&mut self, items: Vec<AiTimelineItem>) -> bool {
            let current = self.deferred_items.as_ref().unwrap_or(&self.items);
            if current == &items {
                return false;
            }
            self.queue_deferred_items(items);
            true
        }

        pub fn sync_queue_items(&mut self, queue_items: Vec<AiTimelineItem>) -> (bool, usize) {
            let mut authoritative_len = self
                .items
                .iter()
                .position(|item| item.kind == "queuedMessage")
                .unwrap_or(self.items.len());
            let hidden_authoritative_rows = authoritative_len
                .saturating_add(queue_items.len())
                .saturating_sub(AI_TIMELINE_MAX_VISIBLE_ROWS)
                .min(authoritative_len);
            let mut changed = hidden_authoritative_rows > 0;
            for _ in 0..hidden_authoritative_rows {
                self.remove(0);
                authoritative_len -= 1;
            }

            let stable_queue_rows = self.items[authoritative_len..]
                .iter()
                .zip(&queue_items)
                .take_while(|(current, next)| current.row_id == next.row_id)
                .count();
            for (offset, item) in queue_items.iter().take(stable_queue_rows).enumerate() {
                let index = authoritative_len + offset;
                if self.items[index] != *item {
                    let _ = self.set(index, item.clone());
                    changed = true;
                }
            }
            while self.items.len() > authoritative_len + stable_queue_rows {
                let _ = self.pop();
                changed = true;
            }
            for item in queue_items.into_iter().skip(stable_queue_rows) {
                self.push(item);
                changed = true;
            }

            (changed, hidden_authoritative_rows)
        }

        pub fn defer_sync_queue_items(
            &mut self,
            queue_items: Vec<AiTimelineItem>,
        ) -> (bool, usize) {
            let current = self.deferred_items.as_ref().unwrap_or(&self.items);
            let authoritative_len = current
                .iter()
                .position(|item| item.kind == "queuedMessage")
                .unwrap_or(current.len());
            let hidden_authoritative_rows = authoritative_len
                .saturating_add(queue_items.len())
                .saturating_sub(AI_TIMELINE_MAX_VISIBLE_ROWS)
                .min(authoritative_len);
            let retained_start = hidden_authoritative_rows;
            let mut items = current[retained_start..authoritative_len].to_vec();
            items.extend(queue_items);
            if current == &items {
                return (false, hidden_authoritative_rows);
            }
            self.queue_deferred_items(items);
            (true, hidden_authoritative_rows)
        }

        pub fn replace(&mut self, items: Vec<AiTimelineItem>) {
            self.replacement = Some(items);
            self.reset();
        }

        pub fn defer_replace(&mut self, items: Vec<AiTimelineItem>) {
            self.queue_deferred_items(items);
        }

        fn queue_deferred_items(&mut self, items: Vec<AiTimelineItem>) {
            self.deferred_items = Some(items);
            if self.deferred_update_scheduled {
                return;
            }
            self.deferred_update_scheduled = true;
            if !self
                .get_qml_method_invoker()
                .invoke_method("apply_deferred_items")
            {
                self.deferred_update_scheduled = false;
            }
        }

        #[qslot]
        fn apply_deferred_items(&mut self) {
            self.deferred_update_scheduled = false;
            let Some(items) = self.deferred_items.take() else {
                return;
            };
            self.sync(items);
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

        fn push_unnotified(&mut self, value: Self::Item) {
            self.items.push(value);
        }

        fn pop_unnotified(&mut self) -> Option<Self::Item> {
            self.items.pop()
        }

        fn remove_unnotified(&mut self, index: usize) -> Self::Item {
            self.items.remove(index)
        }

        fn reset_unnotified(&mut self) {
            self.items = self.replacement.take().unwrap_or_default();
        }
    }
}

pub use timeline_model::AiTimelineListModel;
