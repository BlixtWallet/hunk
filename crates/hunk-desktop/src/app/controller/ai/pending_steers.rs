#[cfg(test)]
fn ai_pending_steer_seed_content(prompt: &str, local_images: &[PathBuf]) -> Option<String> {
    let prompt = prompt.trim();
    let images = local_images
        .iter()
        .map(|path| ai_pending_steer_local_image_name(path.as_path()))
        .collect::<Vec<_>>();

    if prompt.is_empty() && images.is_empty() {
        return None;
    }

    if images.is_empty() {
        return Some(prompt.to_string());
    }

    let image_prefix = if images.len() == 1 {
        "[image] "
    } else {
        "[images] "
    };
    let image_summary = format!("{image_prefix}{}", images.join(", "));
    if prompt.is_empty() {
        Some(image_summary)
    } else {
        Some(format!("{prompt}\n{image_summary}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AiUserMessageSignature {
    text: String,
    images: Vec<String>,
}

fn ai_user_message_signature_from_prompt_and_images(
    prompt: &str,
    local_images: &[PathBuf],
) -> Option<AiUserMessageSignature> {
    let text = prompt.replace("\r\n", "\n").trim().to_string();
    let images = local_images
        .iter()
        .map(|path| ai_pending_steer_local_image_name(path.as_path()))
        .collect::<Vec<_>>();

    if text.is_empty() && images.is_empty() {
        return None;
    }

    Some(AiUserMessageSignature { text, images })
}

fn ai_user_message_signature_from_content(content: &str) -> Option<AiUserMessageSignature> {
    let normalized = content.replace("\r\n", "\n");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut lines = trimmed.lines().collect::<Vec<_>>();
    let images = lines
        .last()
        .and_then(|line| parse_ai_user_message_image_summary_line(line.trim()))
        .unwrap_or_default();
    if !images.is_empty() {
        lines.pop();
    }

    let text = lines.join("\n").trim().to_string();
    Some(AiUserMessageSignature { text, images })
}

fn parse_ai_user_message_image_summary_line(line: &str) -> Option<Vec<String>> {
    let image_list = line
        .strip_prefix("[image] ")
        .or_else(|| line.strip_prefix("[images] "))?;
    let images = image_list
        .split(',')
        .map(str::trim)
        .filter(|image| !image.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    (!images.is_empty()).then_some(images)
}

fn ai_user_message_matches_pending_input(
    content: &str,
    prompt: &str,
    local_images: &[PathBuf],
) -> bool {
    let Some(expected) = ai_user_message_signature_from_prompt_and_images(prompt, local_images)
    else {
        return false;
    };
    let Some(actual) = ai_user_message_signature_from_content(content) else {
        return false;
    };

    actual == expected
}

fn ai_pending_steer_row_id(pending: &AiPendingSteer, pending_ix: usize) -> String {
    format!(
        "pending-steer\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{pending_ix}",
        pending.thread_id, pending.turn_id, pending.accepted_after_sequence
    )
}

fn ai_pending_steer_local_image_name(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn pending_steer_turn_is_in_progress(state: &hunk_codex::state::AiState, pending: &AiPendingSteer) -> bool {
    state
        .turns
        .get(hunk_codex::state::turn_storage_key(
            pending.thread_id.as_str(),
            pending.turn_id.as_str(),
        ).as_str())
        .is_some_and(|turn| turn.status == TurnStatus::InProgress)
}

fn pending_steer_matching_sequence(
    state: &hunk_codex::state::AiState,
    pending: &AiPendingSteer,
    min_sequence: u64,
) -> Option<u64> {
    state
        .items
        .values()
        .filter(|item| {
            item.thread_id == pending.thread_id
                && item.turn_id == pending.turn_id
                && item.kind == "userMessage"
                && ai_user_message_matches_pending_input(
                    item.content.as_str(),
                    pending.prompt.as_str(),
                    pending.local_images.as_slice(),
                )
                && item.last_sequence > min_sequence
        })
        .map(|item| item.last_sequence)
        .min()
}

fn reconcile_ai_pending_steers(
    pending_steers: &mut Vec<AiPendingSteer>,
    state: &hunk_codex::state::AiState,
) {
    if pending_steers.is_empty() {
        return;
    }

    let mut matched_sequence_by_turn = BTreeMap::<(String, String), u64>::new();
    let mut blocked_turns = BTreeSet::<(String, String)>::new();
    let mut unmatched = Vec::with_capacity(pending_steers.len());

    for pending in pending_steers.drain(..) {
        let turn_key = (pending.thread_id.clone(), pending.turn_id.clone());
        if blocked_turns.contains(&turn_key) {
            unmatched.push(pending);
            continue;
        }

        let min_sequence = matched_sequence_by_turn
            .get(&turn_key)
            .copied()
            .unwrap_or(pending.accepted_after_sequence);

        if let Some(sequence) = pending_steer_matching_sequence(state, &pending, min_sequence) {
            matched_sequence_by_turn.insert(turn_key, sequence);
        } else {
            blocked_turns.insert(turn_key);
            unmatched.push(pending);
        }
    }

    *pending_steers = unmatched;
}

fn take_restorable_ai_pending_steers(
    pending_steers: &mut Vec<AiPendingSteer>,
    state: &hunk_codex::state::AiState,
) -> Vec<AiPendingSteer> {
    let mut restorable = Vec::new();
    let mut remaining = Vec::with_capacity(pending_steers.len());

    for pending in pending_steers.drain(..) {
        if pending_steer_turn_is_in_progress(state, &pending) {
            remaining.push(pending);
        } else {
            restorable.push(pending);
        }
    }

    *pending_steers = remaining;
    restorable
}

fn take_all_ai_pending_steers(pending_steers: &mut Vec<AiPendingSteer>) -> Vec<AiPendingSteer> {
    std::mem::take(pending_steers)
}

fn merge_restored_ai_prompt(existing: &mut String, prompt: &str) -> Option<usize> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return None;
    }

    if existing.trim().is_empty() {
        *existing = prompt.to_string();
        return Some(0);
    }

    let mut existing_matches = existing.match_indices(prompt).map(|(offset, _)| offset);
    match (existing_matches.next(), existing_matches.next()) {
        (Some(offset), None) => return Some(offset),
        (Some(_), Some(_)) => return None,
        (None, _) => {}
    }

    let insertion_offset = existing.len() + 2;
    existing.push_str("\n\n");
    existing.push_str(prompt);
    Some(insertion_offset)
}

impl DiffViewer {
    pub(crate) fn ai_pending_steer_row_ids_for_thread(&self, thread_id: &str) -> Vec<String> {
        self.ai_pending_steers
            .iter()
            .enumerate()
            .filter(|(_, pending)| pending.thread_id == thread_id)
            .map(|(pending_ix, pending)| ai_pending_steer_row_id(pending, pending_ix))
            .collect()
    }

    pub(crate) fn ai_pending_steer_for_row_id(&self, row_id: &str) -> Option<AiPendingSteer> {
        self.ai_pending_steers
            .iter()
            .enumerate()
            .find_map(|(pending_ix, pending)| {
                (ai_pending_steer_row_id(pending, pending_ix) == row_id).then(|| pending.clone())
            })
    }

    fn restore_ai_pending_steers_to_drafts(
        &mut self,
        pending_steers: Vec<AiPendingSteer>,
    ) -> BTreeSet<AiComposerDraftKey> {
        let mut touched = BTreeSet::new();

        for pending in pending_steers {
            let target_key = AiComposerDraftKey::Thread(pending.thread_id.clone());
            let draft = self.ai_composer_drafts.entry(target_key.clone()).or_default();
            let restored_prompt_offset =
                merge_restored_ai_prompt(&mut draft.prompt, pending.prompt.as_str());
            for image_path in pending.local_images {
                if !draft.local_images.contains(&image_path) {
                    draft.local_images.push(image_path);
                }
            }
            crate::app::ai_composer_completion::merge_rebased_ai_composer_skill_bindings(
                &mut draft.skill_bindings,
                pending.skill_bindings.as_slice(),
                restored_prompt_offset,
                draft.prompt.as_str(),
            );
            touched.insert(target_key);
        }

        touched
    }

    fn restore_all_visible_ai_pending_steers_to_drafts(&mut self) -> BTreeSet<AiComposerDraftKey> {
        let pending_steers = take_all_ai_pending_steers(&mut self.ai_pending_steers);
        self.restore_ai_pending_steers_to_drafts(pending_steers)
    }
}
