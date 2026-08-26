use std::cmp::Ordering;
use std::ops::Range;

use serde_json::json;

const FILE_COMPLETION_LIMIT: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlashCommandAvailability {
    Always,
    IdleOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SlashCommand {
    pub name: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub availability: SlashCommandAvailability,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComposerCompletion {
    pub kind: &'static str,
    pub value: String,
    pub label: String,
    pub description: String,
    pub disabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActiveToken {
    query: String,
    replace_range: Range<usize>,
}

const SLASH_COMMANDS: [SlashCommand; 8] = [
    SlashCommand {
        name: "code",
        label: "Code",
        description: "Switch to standard coding mode.",
        availability: SlashCommandAvailability::IdleOnly,
    },
    SlashCommand {
        name: "plan",
        label: "Plan",
        description: "Switch to planning mode before coding.",
        availability: SlashCommandAvailability::IdleOnly,
    },
    SlashCommand {
        name: "review",
        label: "Review",
        description: "Open the diff review workspace.",
        availability: SlashCommandAvailability::IdleOnly,
    },
    SlashCommand {
        name: "fast-mode-on",
        label: "Fast Mode On",
        description: "Use the Fast service tier for quicker responses.",
        availability: SlashCommandAvailability::IdleOnly,
    },
    SlashCommand {
        name: "fast-mode-off",
        label: "Fast Mode Off",
        description: "Switch back to the Standard service tier.",
        availability: SlashCommandAvailability::IdleOnly,
    },
    SlashCommand {
        name: "status",
        label: "Status",
        description: "Show the current Codex session status.",
        availability: SlashCommandAvailability::Always,
    },
    SlashCommand {
        name: "login",
        label: "Login",
        description: "Start ChatGPT login for this workspace.",
        availability: SlashCommandAvailability::Always,
    },
    SlashCommand {
        name: "logout",
        label: "Logout",
        description: "Disconnect the current account.",
        availability: SlashCommandAvailability::Always,
    },
];

pub fn composer_completions(
    prompt: &str,
    cursor_position_utf16: usize,
    task_in_progress: bool,
    visible_paths: &[String],
) -> Vec<ComposerCompletion> {
    let cursor = utf16_position_to_byte_offset(prompt, cursor_position_utf16);
    if let Some(token) = active_slash_token(prompt, cursor) {
        return matching_slash_commands(token.query.as_str(), task_in_progress);
    }
    let Some(token) = active_file_token(prompt, cursor) else {
        return Vec::new();
    };
    if token.query.is_empty() {
        return Vec::new();
    }

    matching_file_paths(visible_paths, token.query.as_str(), FILE_COMPLETION_LIMIT)
        .into_iter()
        .map(|path| ComposerCompletion {
            kind: "file",
            value: path.clone(),
            label: file_name(path.as_str()).to_owned(),
            description: path,
            disabled: false,
        })
        .collect()
}

pub fn composer_completions_json(
    prompt: &str,
    cursor_position_utf16: usize,
    task_in_progress: bool,
    visible_paths: &[String],
) -> String {
    let items = composer_completions(
        prompt,
        cursor_position_utf16,
        task_in_progress,
        visible_paths,
    );
    serde_json::to_string(
        &items
            .iter()
            .map(|item| {
                json!({
                    "kind": item.kind,
                    "value": item.value,
                    "label": item.label,
                    "description": item.description,
                    "disabled": item.disabled,
                })
            })
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".to_owned())
}

pub fn prompt_after_completion(
    prompt: &str,
    cursor_position_utf16: usize,
    kind: &str,
    value: &str,
) -> Option<String> {
    let cursor = utf16_position_to_byte_offset(prompt, cursor_position_utf16);
    let (token, inserted_text) = match kind {
        "command" => {
            if !SLASH_COMMANDS.iter().any(|command| command.name == value) {
                return None;
            }
            (active_slash_token(prompt, cursor)?, String::new())
        }
        "file" => {
            let token = active_file_token(prompt, cursor)?;
            let trailing_space = !prompt[token.replace_range.end..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace);
            (token, inserted_file_text(value, trailing_space))
        }
        _ => return None,
    };

    let mut next = String::with_capacity(
        prompt
            .len()
            .saturating_sub(token.replace_range.len())
            .saturating_add(inserted_text.len()),
    );
    next.push_str(&prompt[..token.replace_range.start]);
    next.push_str(inserted_text.as_str());
    next.push_str(&prompt[token.replace_range.end..]);
    if kind == "command" {
        Some(next.trim_start().to_owned())
    } else {
        Some(next)
    }
}

pub fn slash_command(name: &str) -> Option<SlashCommand> {
    SLASH_COMMANDS
        .iter()
        .copied()
        .find(|command| command.name == name)
}

fn matching_slash_commands(query: &str, task_in_progress: bool) -> Vec<ComposerCompletion> {
    let normalized_query = query.trim().to_ascii_lowercase();
    let mut matches = SLASH_COMMANDS
        .iter()
        .filter_map(|command| {
            let name = command.name.to_ascii_lowercase();
            let label = command.label.to_ascii_lowercase();
            let description = command.description.to_ascii_lowercase();
            let score = if normalized_query.is_empty() {
                0
            } else {
                match_score(name.as_str(), normalized_query.as_str())
                    .max(match_score(label.as_str(), normalized_query.as_str()))
                    .max(if description.contains(normalized_query.as_str()) {
                        1_000
                    } else {
                        i32::MIN
                    })
            };
            (score != i32::MIN).then_some((*command, score))
        })
        .collect::<Vec<_>>();
    if !normalized_query.is_empty() {
        matches.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| left.0.name.cmp(right.0.name))
        });
    }
    matches
        .into_iter()
        .map(|(command, _)| ComposerCompletion {
            kind: "command",
            value: command.name.to_owned(),
            label: format!("/{}", command.name),
            description: command.description.to_owned(),
            disabled: task_in_progress
                && command.availability == SlashCommandAvailability::IdleOnly,
        })
        .collect()
}

fn matching_file_paths(paths: &[String], query: &str, limit: usize) -> Vec<String> {
    let normalized_query = normalize_match_key(query);
    let mut matches = paths
        .iter()
        .filter_map(|path| {
            let normalized_path = normalize_match_key(path.as_str());
            let normalized_name = normalize_match_key(file_name(path.as_str()));
            file_match_score(
                normalized_path.as_str(),
                normalized_name.as_str(),
                normalized_query.as_str(),
            )
            .map(|score| (path, score))
        })
        .collect::<Vec<_>>();
    matches.sort_by(compare_file_matches);
    matches
        .into_iter()
        .take(limit)
        .map(|(path, _)| path.clone())
        .collect()
}

fn compare_file_matches(left: &(&String, i32), right: &(&String, i32)) -> Ordering {
    right
        .1
        .cmp(&left.1)
        .then_with(|| left.0.len().cmp(&right.0.len()))
        .then_with(|| left.0.cmp(right.0))
}

fn file_match_score(path: &str, name: &str, query: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    if path == query {
        return Some(10_000);
    }
    if name == query {
        return Some(9_600);
    }
    if name.starts_with(query) {
        return Some(8_900 - length_penalty(name, query));
    }
    if let Some(position) = name.find(query) {
        return Some(8_000 - position as i32 * 12 - length_penalty(name, query));
    }
    if path.starts_with(query) {
        return Some(7_600 - length_penalty(path, query));
    }
    if let Some(position) = segment_prefix_position(path, query) {
        return Some(7_200 - position as i32 * 8 - length_penalty(path, query));
    }
    if let Some(position) = path.find(query) {
        return Some(6_400 - position as i32 * 10 - length_penalty(path, query));
    }
    subsequence_match_score(path, query)
}

fn match_score(candidate: &str, query: &str) -> i32 {
    if candidate.starts_with(query) {
        8_000 - length_penalty(candidate, query)
    } else {
        subsequence_match_score(candidate, query).unwrap_or(i32::MIN)
    }
}

fn subsequence_match_score(candidate: &str, query: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let candidate = candidate.as_bytes();
    let query = query.as_bytes();
    let mut query_index = 0;
    let mut score = 2_000;
    let mut previous_match = None;

    for (candidate_index, byte) in candidate.iter().copied().enumerate() {
        if byte != query[query_index] {
            continue;
        }
        score += 18;
        if candidate_index == 0 || is_match_boundary(candidate[candidate_index - 1]) {
            score += 30;
        }
        if previous_match.is_some_and(|previous| candidate_index == previous + 1) {
            score += 24;
        }
        previous_match = Some(candidate_index);
        query_index += 1;
        if query_index == query.len() {
            return Some(score - length_penalty_bytes(candidate.len(), query.len()));
        }
    }
    None
}

fn active_slash_token(text: &str, cursor: usize) -> Option<ActiveToken> {
    let leading_whitespace = text
        .chars()
        .take_while(|character| character.is_whitespace())
        .map(char::len_utf8)
        .sum::<usize>();
    if cursor < leading_whitespace {
        return None;
    }
    let token_end = leading_whitespace
        + text[leading_whitespace..]
            .char_indices()
            .find(|(_, character)| character.is_whitespace())
            .map(|(index, _)| index)
            .unwrap_or(text.len().saturating_sub(leading_whitespace));
    if token_end <= leading_whitespace || cursor > token_end {
        return None;
    }
    let token = &text[leading_whitespace..token_end];
    let query = token.strip_prefix('/')?;
    if query
        .chars()
        .any(|character| !(character.is_ascii_alphanumeric() || character == '-'))
    {
        return None;
    }
    Some(ActiveToken {
        query: query.to_owned(),
        replace_range: leading_whitespace..token_end,
    })
}

fn active_file_token(text: &str, cursor: usize) -> Option<ActiveToken> {
    let token_start = text[..cursor]
        .char_indices()
        .rfind(|(_, character)| character.is_whitespace())
        .map(|(index, character)| index + character.len_utf8())
        .unwrap_or(0);
    let token_end = cursor
        + text[cursor..]
            .char_indices()
            .find(|(_, character)| character.is_whitespace())
            .map(|(index, _)| index)
            .unwrap_or(text.len().saturating_sub(cursor));
    let token = &text[token_start..token_end];
    let query = token.strip_prefix('@')?;
    Some(ActiveToken {
        query: query.to_owned(),
        replace_range: token_start..token_end,
    })
}

fn inserted_file_text(path: &str, trailing_space: bool) -> String {
    let suffix = if trailing_space { " " } else { "" };
    if path.chars().any(char::is_whitespace) && !path.contains('"') {
        format!("\"{path}\"{suffix}")
    } else {
        format!("{path}{suffix}")
    }
}

fn utf16_position_to_byte_offset(text: &str, position: usize) -> usize {
    let mut utf16_offset = 0;
    for (byte_offset, character) in text.char_indices() {
        let next_offset = utf16_offset + character.len_utf16();
        if next_offset > position {
            return byte_offset;
        }
        utf16_offset = next_offset;
    }
    text.len()
}

fn segment_prefix_position(candidate: &str, query: &str) -> Option<usize> {
    let mut offset = 0;
    for segment in candidate.split('/') {
        if segment.starts_with(query) {
            return Some(offset);
        }
        offset += segment.len() + 1;
    }
    None
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn normalize_match_key(value: &str) -> String {
    value.trim().to_lowercase().replace('\\', "/")
}

fn length_penalty(candidate: &str, query: &str) -> i32 {
    length_penalty_bytes(candidate.len(), query.len())
}

fn length_penalty_bytes(candidate: usize, query: usize) -> i32 {
    i32::try_from(candidate.saturating_sub(query)).unwrap_or(i32::MAX)
}

fn is_match_boundary(byte: u8) -> bool {
    matches!(byte, b'/' | b'-' | b'_' | b'.')
}
