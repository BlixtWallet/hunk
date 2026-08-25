use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use hunk_codex::state::{AiState, ThreadLifecycleStatus, TurnStatus};

use crate::AI_PROMPT_MAX_ATTACHMENTS;
use crate::AiTimelineItem;

pub const AI_MESSAGE_QUEUE_MAX_ITEMS: usize = 64;
pub const AI_MESSAGE_QUEUE_MAX_PROMPT_BYTES: usize = 256 * 1024;
pub const AI_MESSAGE_QUEUE_MAX_RETAINED_BYTES: usize = 1024 * 1024;
const AI_QUEUE_RECEIPTS_PER_THREAD: usize = 8;
const AI_QUEUE_VISIBLE_TEXT_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct AiPromptFingerprint {
    byte_len: usize,
    hash: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AiQueueThreadProjection {
    pub accepts_messages: bool,
    pub running: bool,
    pub latest_sequence: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct AiQueueUserMessageReceipt {
    thread_id: String,
    fingerprint: AiPromptFingerprint,
    sequence: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AiQueueProjection {
    pub threads: BTreeMap<String, AiQueueThreadProjection>,
    user_messages: Vec<AiQueueUserMessageReceipt>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AiQueuedMessageStatus {
    Queued,
    PendingConfirmation { accepted_after_sequence: u64 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AiQueuedMessage {
    id: u64,
    thread_id: String,
    prompt: String,
    local_image_paths: Vec<PathBuf>,
    fingerprint: AiPromptFingerprint,
    status: AiQueuedMessageStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AiQueuedMessageCommand {
    pub thread_id: String,
    pub prompt: String,
    pub local_image_paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AiRecoveredDraft {
    pub prompt: String,
    pub local_image_paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AiRecoveredMessage {
    prompt: String,
    local_image_paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AiMessageQueue {
    messages: Vec<AiQueuedMessage>,
    recovered_messages: BTreeMap<String, Vec<AiRecoveredMessage>>,
    interrupt_restore_thread_ids: BTreeSet<String>,
    next_id: u64,
}

impl AiQueueProjection {
    pub fn from_state(state: &AiState, visible_thread_ids: &[&str]) -> Self {
        let visible_thread_ids = visible_thread_ids.iter().copied().collect::<BTreeSet<_>>();
        let mut threads = BTreeMap::new();
        for thread_id in &visible_thread_ids {
            let Some(thread) = state.threads.get(*thread_id) else {
                continue;
            };
            threads.insert(
                (*thread_id).to_owned(),
                AiQueueThreadProjection {
                    accepts_messages: matches!(
                        thread.status,
                        ThreadLifecycleStatus::Active | ThreadLifecycleStatus::Idle
                    ),
                    running: state.turns.values().any(|turn| {
                        turn.thread_id == *thread_id && turn.status == TurnStatus::InProgress
                    }),
                    latest_sequence: thread_latest_timeline_sequence(state, thread_id),
                },
            );
        }

        let mut receipts_by_thread = BTreeMap::<String, Vec<AiQueueUserMessageReceipt>>::new();
        for item in state.items.values().filter(|item| {
            item.kind == "userMessage" && visible_thread_ids.contains(item.thread_id.as_str())
        }) {
            receipts_by_thread
                .entry(item.thread_id.clone())
                .or_default()
                .push(AiQueueUserMessageReceipt {
                    thread_id: item.thread_id.clone(),
                    fingerprint: prompt_fingerprint(item.content.as_str()),
                    sequence: item.last_sequence,
                });
        }
        let mut user_messages = Vec::new();
        for receipts in receipts_by_thread.values_mut() {
            receipts.sort_by_key(|receipt| receipt.sequence);
            let keep_from = receipts.len().saturating_sub(AI_QUEUE_RECEIPTS_PER_THREAD);
            user_messages.extend(receipts.drain(keep_from..));
        }

        Self {
            threads,
            user_messages,
        }
    }

    fn matching_sequence(
        &self,
        thread_id: &str,
        fingerprint: AiPromptFingerprint,
        min_sequence: u64,
    ) -> Option<u64> {
        self.user_messages
            .iter()
            .filter(|receipt| {
                receipt.thread_id == thread_id
                    && receipt.fingerprint == fingerprint
                    && receipt.sequence > min_sequence
            })
            .map(|receipt| receipt.sequence)
            .min()
    }
}

impl AiMessageQueue {
    pub fn enqueue(&mut self, thread_id: String, prompt: String) -> Result<(), &'static str> {
        self.enqueue_with_attachments(thread_id, prompt, Vec::new())
    }

    pub fn enqueue_with_attachments(
        &mut self,
        thread_id: String,
        prompt: String,
        local_image_paths: Vec<PathBuf>,
    ) -> Result<(), &'static str> {
        let thread_id = thread_id.trim();
        let prompt = prompt.trim();
        if thread_id.is_empty() || (prompt.is_empty() && local_image_paths.is_empty()) {
            return Err("A queued follow-up requires an active thread and message.");
        }
        if local_image_paths.len() > AI_PROMPT_MAX_ATTACHMENTS {
            return Err("The queued follow-up has too many image attachments.");
        }
        if self.retained_message_count() >= AI_MESSAGE_QUEUE_MAX_ITEMS {
            return Err("The queued follow-up limit has been reached.");
        }
        if prompt.len() > AI_MESSAGE_QUEUE_MAX_PROMPT_BYTES {
            return Err("The queued follow-up is too large.");
        }
        if self
            .retained_bytes()
            .saturating_add(retained_message_cost(prompt, local_image_paths.as_slice()))
            > AI_MESSAGE_QUEUE_MAX_RETAINED_BYTES
        {
            return Err("The queued follow-ups are using too much memory.");
        }
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.messages.push(AiQueuedMessage {
            id: self.next_id,
            thread_id: thread_id.to_owned(),
            prompt: prompt.to_owned(),
            fingerprint: prompt_input_fingerprint(prompt, local_image_paths.as_slice()),
            local_image_paths,
            status: AiQueuedMessageStatus::Queued,
        });
        Ok(())
    }

    pub fn edit_latest_with_attachments(
        &mut self,
        thread_id: &str,
    ) -> Option<AiQueuedMessageCommand> {
        let index = self.messages.iter().rposition(|message| {
            message.thread_id == thread_id && message.status == AiQueuedMessageStatus::Queued
        })?;
        let message = self.messages.remove(index);
        Some(AiQueuedMessageCommand {
            thread_id: message.thread_id,
            prompt: message.prompt,
            local_image_paths: message.local_image_paths,
        })
    }

    pub fn mark_interrupt_restore(&mut self, thread_id: String) {
        if self.thread_count(thread_id.as_str()) > 0 {
            self.interrupt_restore_thread_ids.insert(thread_id);
        }
    }

    pub fn reconcile(&mut self, projection: &AiQueueProjection) -> bool {
        let mut changed = self.reconcile_confirmed(projection);
        let mut recover_thread_ids = self
            .messages
            .iter()
            .filter(|message| {
                projection
                    .threads
                    .get(message.thread_id.as_str())
                    .is_some_and(|thread| !thread.accepts_messages)
            })
            .map(|message| message.thread_id.clone())
            .collect::<BTreeSet<_>>();
        recover_thread_ids.extend(
            self.interrupt_restore_thread_ids
                .iter()
                .filter(|thread_id| {
                    projection
                        .threads
                        .get(thread_id.as_str())
                        .is_none_or(|thread| !thread.running)
                })
                .cloned(),
        );
        if !recover_thread_ids.is_empty() {
            changed |= self.recover_threads(&recover_thread_ids);
            self.interrupt_restore_thread_ids
                .retain(|thread_id| !recover_thread_ids.contains(thread_id));
        }
        changed
    }

    pub fn ready_thread_ids(
        &self,
        projection: &AiQueueProjection,
        blocked_thread_ids: &BTreeSet<String>,
    ) -> Vec<String> {
        let mut ready = Vec::new();
        let mut visited = BTreeSet::new();
        for message in &self.messages {
            if !visited.insert(message.thread_id.clone())
                || blocked_thread_ids.contains(message.thread_id.as_str())
                || self
                    .interrupt_restore_thread_ids
                    .contains(message.thread_id.as_str())
            {
                continue;
            }
            let Some(thread) = projection.threads.get(message.thread_id.as_str()) else {
                continue;
            };
            if thread.accepts_messages
                && !thread.running
                && message.status == AiQueuedMessageStatus::Queued
            {
                ready.push(message.thread_id.clone());
            }
        }
        ready
    }

    pub fn mark_next_pending(
        &mut self,
        thread_id: &str,
        accepted_after_sequence: u64,
    ) -> Option<AiQueuedMessageCommand> {
        let message = self.messages.iter_mut().find(|message| {
            message.thread_id == thread_id && message.status == AiQueuedMessageStatus::Queued
        })?;
        message.status = AiQueuedMessageStatus::PendingConfirmation {
            accepted_after_sequence,
        };
        Some(AiQueuedMessageCommand {
            thread_id: message.thread_id.clone(),
            prompt: message.prompt.clone(),
            local_image_paths: message.local_image_paths.clone(),
        })
    }

    pub fn accept_steer(
        &mut self,
        thread_id: &str,
        prompt: &str,
        local_image_paths: &[PathBuf],
    ) -> bool {
        let fingerprint = prompt_input_fingerprint(prompt, local_image_paths);
        let Some(index) = self.messages.iter().position(|message| {
            message.thread_id == thread_id
                && message.fingerprint == fingerprint
                && matches!(
                    message.status,
                    AiQueuedMessageStatus::PendingConfirmation { .. }
                )
        }) else {
            return false;
        };
        self.messages.remove(index);
        true
    }

    pub fn reset_pending_after_runtime_failure(&mut self) -> bool {
        let mut changed = false;
        for message in &mut self.messages {
            if matches!(
                message.status,
                AiQueuedMessageStatus::PendingConfirmation { .. }
            ) {
                message.status = AiQueuedMessageStatus::Queued;
                changed = true;
            }
        }
        changed
    }

    pub fn timeline_items(&self, thread_id: &str) -> Vec<AiTimelineItem> {
        self.messages
            .iter()
            .filter(|message| message.thread_id == thread_id)
            .map(|message| {
                let content = prompt_input_content(
                    message.prompt.as_str(),
                    message.local_image_paths.as_slice(),
                );
                let (text, truncated) = bounded_text(content.as_str(), AI_QUEUE_VISIBLE_TEXT_BYTES);
                let sending = matches!(
                    message.status,
                    AiQueuedMessageStatus::PendingConfirmation { .. }
                );
                AiTimelineItem {
                    row_id: format!("queued-message:{}", message.id),
                    kind: "queuedMessage".to_owned(),
                    role: "user".to_owned(),
                    title: "You".to_owned(),
                    text,
                    status: if sending { "sending" } else { "queued" }.to_owned(),
                    streaming: sending,
                    truncated,
                    ..AiTimelineItem::default()
                }
            })
            .collect()
    }

    pub fn take_recovered_draft(&mut self, thread_id: &str) -> AiRecoveredDraft {
        let messages = self
            .recovered_messages
            .remove(thread_id)
            .unwrap_or_default();
        let prompt = messages
            .iter()
            .map(|message| message.prompt.trim())
            .filter(|prompt| !prompt.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        let mut local_image_paths = Vec::new();
        for path in messages
            .into_iter()
            .flat_map(|message| message.local_image_paths)
        {
            if !local_image_paths.contains(&path) {
                local_image_paths.push(path);
            }
        }
        AiRecoveredDraft {
            prompt,
            local_image_paths,
        }
    }

    pub fn total_count(&self) -> usize {
        self.messages.len()
    }

    pub fn thread_count(&self, thread_id: &str) -> usize {
        self.messages
            .iter()
            .filter(|message| message.thread_id == thread_id)
            .count()
    }

    pub fn thread_is_sending(&self, thread_id: &str) -> bool {
        self.messages.iter().any(|message| {
            message.thread_id == thread_id
                && matches!(
                    message.status,
                    AiQueuedMessageStatus::PendingConfirmation { .. }
                )
        })
    }

    pub fn next_queued_has_attachments(&self, thread_id: &str) -> bool {
        self.messages
            .iter()
            .find(|message| message.thread_id == thread_id)
            .is_some_and(|message| {
                message.status == AiQueuedMessageStatus::Queued
                    && !message.local_image_paths.is_empty()
            })
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.recovered_messages.clear();
        self.interrupt_restore_thread_ids.clear();
    }

    fn retained_message_count(&self) -> usize {
        self.messages.len()
            + self
                .recovered_messages
                .values()
                .map(Vec::len)
                .sum::<usize>()
    }

    fn retained_bytes(&self) -> usize {
        self.messages
            .iter()
            .map(|message| {
                retained_message_cost(
                    message.prompt.as_str(),
                    message.local_image_paths.as_slice(),
                )
            })
            .chain(self.recovered_messages.values().flatten().map(|message| {
                retained_message_cost(
                    message.prompt.as_str(),
                    message.local_image_paths.as_slice(),
                )
            }))
            .fold(0usize, usize::saturating_add)
    }

    fn reconcile_confirmed(&mut self, projection: &AiQueueProjection) -> bool {
        let mut matched_sequence_by_thread = BTreeMap::<String, u64>::new();
        let mut blocked_thread_ids = BTreeSet::<String>::new();
        let mut remaining = Vec::with_capacity(self.messages.len());
        let original_len = self.messages.len();

        for message in self.messages.drain(..) {
            let AiQueuedMessageStatus::PendingConfirmation {
                accepted_after_sequence,
            } = message.status
            else {
                remaining.push(message);
                continue;
            };
            if blocked_thread_ids.contains(message.thread_id.as_str()) {
                remaining.push(message);
                continue;
            }
            let min_sequence = matched_sequence_by_thread
                .get(message.thread_id.as_str())
                .copied()
                .unwrap_or(accepted_after_sequence);
            if let Some(sequence) = projection.matching_sequence(
                message.thread_id.as_str(),
                message.fingerprint,
                min_sequence,
            ) {
                matched_sequence_by_thread.insert(message.thread_id.clone(), sequence);
            } else {
                blocked_thread_ids.insert(message.thread_id.clone());
                remaining.push(message);
            }
        }
        self.messages = remaining;
        self.messages.len() != original_len
    }

    fn recover_threads(&mut self, thread_ids: &BTreeSet<String>) -> bool {
        let mut remaining = Vec::with_capacity(self.messages.len());
        let mut recovered = false;
        for message in self.messages.drain(..) {
            if thread_ids.contains(message.thread_id.as_str()) {
                self.recovered_messages
                    .entry(message.thread_id)
                    .or_default()
                    .push(AiRecoveredMessage {
                        prompt: message.prompt,
                        local_image_paths: message.local_image_paths,
                    });
                recovered = true;
            } else {
                remaining.push(message);
            }
        }
        self.messages = remaining;
        recovered
    }
}

fn retained_message_cost(prompt: &str, local_image_paths: &[PathBuf]) -> usize {
    local_image_paths
        .iter()
        .map(|path| path.as_os_str().len().saturating_add(1))
        .fold(prompt.len().saturating_add(2), usize::saturating_add)
}

fn thread_latest_timeline_sequence(state: &AiState, thread_id: &str) -> u64 {
    let thread_sequence = state
        .threads
        .get(thread_id)
        .map(|thread| thread.last_sequence)
        .unwrap_or(0);
    state
        .turns
        .values()
        .filter(|turn| turn.thread_id == thread_id)
        .map(|turn| turn.last_sequence)
        .chain(
            state
                .items
                .values()
                .filter(|item| item.thread_id == thread_id)
                .map(|item| item.last_sequence),
        )
        .chain(
            state
                .turn_plans
                .values()
                .filter(|plan| plan.thread_id == thread_id)
                .map(|plan| plan.last_sequence),
        )
        .max()
        .map_or(thread_sequence, |sequence| sequence.max(thread_sequence))
}

fn prompt_fingerprint(prompt: &str) -> AiPromptFingerprint {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let bytes = prompt.trim().as_bytes();
    let mut byte_len = 0usize;
    let mut hash = FNV_OFFSET;
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
            index += 2;
            b'\n'
        } else {
            let byte = bytes[index];
            index += 1;
            byte
        };
        byte_len = byte_len.saturating_add(1);
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    AiPromptFingerprint { byte_len, hash }
}

fn prompt_input_fingerprint(prompt: &str, local_image_paths: &[PathBuf]) -> AiPromptFingerprint {
    prompt_fingerprint(prompt_input_content(prompt, local_image_paths).as_str())
}

fn prompt_input_content(prompt: &str, local_image_paths: &[PathBuf]) -> String {
    let prompt = prompt.trim();
    if local_image_paths.is_empty() {
        return prompt.to_owned();
    }
    let prefix = if local_image_paths.len() == 1 {
        "[image]"
    } else {
        "[images]"
    };
    let images = local_image_paths
        .iter()
        .map(|path| local_image_display_name(path.as_path()))
        .collect::<Vec<_>>()
        .join(", ");
    let summary = format!("{prefix} {images}");
    if prompt.is_empty() {
        summary
    } else {
        format!("{prompt}\n{summary}")
    }
}

fn local_image_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn bounded_text(text: &str, max_bytes: usize) -> (String, bool) {
    const ELLIPSIS: &str = "…";
    if text.len() <= max_bytes {
        return (text.to_owned(), false);
    }
    let mut end = max_bytes.saturating_sub(ELLIPSIS.len()).min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut bounded = text[..end].trim_end().to_owned();
    bounded.push_str(ELLIPSIS);
    (bounded, true)
}
