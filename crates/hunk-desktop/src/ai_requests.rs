use std::collections::{BTreeMap, BTreeSet};

use hunk_app::ai::{
    AiApprovalKind, AiPendingApproval, AiPendingUserInputQuestion,
    AiPendingUserInputQuestionOption, AiPendingUserInputRequest,
};

const MAX_QUESTIONS: usize = 8;
const MAX_OPTIONS: usize = 8;
const MAX_ATTENTION_THREADS: usize = 200;
const MAX_HEADER_BYTES: usize = 160;
const MAX_TEXT_BYTES: usize = 2 * 1024;
const MAX_OPTION_BYTES: usize = 512;
const MAX_ANSWER_BYTES: usize = 4 * 1024;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AiPendingRequestProjection {
    pub total_count: i32,
    pub active_count: i32,
    pub approval_count: i32,
    pub input_count: i32,
    pub current: Option<AiPendingRequest>,
    attention_thread_ids: BTreeSet<String>,
    pending_request_ids: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AiPendingRequest {
    pub request_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub kind: String,
    pub title: String,
    pub description: String,
    pub reason: String,
    pub answerable: bool,
    pub questions: Vec<AiPendingQuestion>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AiPendingQuestion {
    pub id: String,
    pub header: String,
    pub question: String,
    pub is_other: bool,
    pub is_secret: bool,
    pub options: Vec<AiPendingOption>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AiPendingOption {
    pub label: String,
    pub description: String,
}

impl AiPendingRequestProjection {
    pub fn from_pending(
        active_thread_id: Option<&str>,
        approvals: &[AiPendingApproval],
        user_inputs: &[AiPendingUserInputRequest],
        visible_thread_ids: &[&str],
    ) -> Self {
        let total_count = approvals.len().saturating_add(user_inputs.len());
        let active_needs_attention = active_thread_id.is_some_and(|active_thread_id| {
            approvals
                .iter()
                .any(|request| request.thread_id == active_thread_id)
                || user_inputs
                    .iter()
                    .any(|request| request.thread_id == active_thread_id)
        });
        let mut attention_thread_ids = BTreeSet::new();
        let mut pending_request_ids = BTreeSet::new();
        if active_needs_attention {
            let active_thread_id = active_thread_id.unwrap_or_default();
            attention_thread_ids.insert(active_thread_id.to_owned());
            if let Some(request_id) = current_request_id(active_thread_id, approvals, user_inputs) {
                pending_request_ids.insert(request_id.to_owned());
            }
        }
        for thread_id in visible_thread_ids {
            if attention_thread_ids.len() >= MAX_ATTENTION_THREADS {
                break;
            }
            let needs_attention = approvals
                .iter()
                .any(|request| request.thread_id == *thread_id)
                || user_inputs
                    .iter()
                    .any(|request| request.thread_id == *thread_id);
            if needs_attention {
                attention_thread_ids.insert((*thread_id).to_owned());
                if let Some(request_id) = current_request_id(thread_id, approvals, user_inputs) {
                    pending_request_ids.insert(request_id.to_owned());
                }
            }
        }
        let active_count = active_thread_id.map_or(0, |active_thread_id| {
            approvals
                .iter()
                .filter(|request| request.thread_id == active_thread_id)
                .count()
                .saturating_add(
                    user_inputs
                        .iter()
                        .filter(|request| request.thread_id == active_thread_id)
                        .count(),
                )
        });
        let current = active_thread_id.and_then(|active_thread_id| {
            approvals
                .iter()
                .find(|request| request.thread_id == active_thread_id)
                .map(project_approval)
                .or_else(|| {
                    user_inputs
                        .iter()
                        .find(|request| request.thread_id == active_thread_id)
                        .map(project_user_input)
                })
        });

        Self {
            total_count: saturating_usize_to_i32(total_count),
            active_count: saturating_usize_to_i32(active_count),
            approval_count: saturating_usize_to_i32(approvals.len()),
            input_count: saturating_usize_to_i32(user_inputs.len()),
            current,
            attention_thread_ids,
            pending_request_ids,
        }
    }

    pub fn attention_thread_ids(&self) -> &BTreeSet<String> {
        &self.attention_thread_ids
    }

    pub fn request_is_pending(&self, request_id: &str) -> bool {
        self.pending_request_ids.contains(request_id)
    }

    pub fn thread_needs_attention(&self, thread_id: &str) -> bool {
        self.attention_thread_ids.contains(thread_id)
    }

    pub fn questions_json(&self) -> String {
        let Some(request) = self.current.as_ref() else {
            return "[]".to_owned();
        };
        serde_json::Value::Array(
            request
                .questions
                .iter()
                .map(|question| {
                    serde_json::json!({
                        "id": question.id,
                        "header": question.header,
                        "question": question.question,
                        "isOther": question.is_other,
                        "isSecret": question.is_secret,
                        "options": question.options.iter().map(|option| {
                            serde_json::json!({
                                "label": option.label,
                                "description": option.description,
                            })
                        }).collect::<Vec<_>>(),
                    })
                })
                .collect(),
        )
        .to_string()
    }

    pub fn validated_answers(
        &self,
        request_id: &str,
        answers_json: &str,
    ) -> Result<BTreeMap<String, Vec<String>>, String> {
        let request = self
            .current
            .as_ref()
            .filter(|request| request.request_id == request_id && request.kind == "userInput")
            .ok_or_else(|| "The pending Codex input request changed.".to_owned())?;
        if !request.answerable {
            return Err("This Codex input request is too large to answer safely in Qt.".to_owned());
        }
        let answers = serde_json::from_str::<BTreeMap<String, Vec<String>>>(answers_json)
            .map_err(|_| "Codex input answers were malformed.".to_owned())?;
        if answers.len() != request.questions.len() {
            return Err("Every Codex question requires one answer.".to_owned());
        }
        for question in &request.questions {
            let values = answers
                .get(question.id.as_str())
                .filter(|values| values.len() == 1)
                .ok_or_else(|| "Every Codex question requires one answer.".to_owned())?;
            let answer = values[0].as_str();
            if answer.len() > MAX_ANSWER_BYTES {
                return Err("A Codex input answer is too long.".to_owned());
            }
            if !question.options.is_empty()
                && !question.is_other
                && !question.options.iter().any(|option| option.label == answer)
            {
                return Err("A Codex input answer is no longer available.".to_owned());
            }
        }
        Ok(answers)
    }
}

fn current_request_id<'a>(
    thread_id: &str,
    approvals: &'a [AiPendingApproval],
    user_inputs: &'a [AiPendingUserInputRequest],
) -> Option<&'a str> {
    approvals
        .iter()
        .find(|request| request.thread_id == thread_id)
        .map(|request| request.request_id.as_str())
        .or_else(|| {
            user_inputs
                .iter()
                .find(|request| request.thread_id == thread_id)
                .map(|request| request.request_id.as_str())
        })
}

fn project_approval(approval: &AiPendingApproval) -> AiPendingRequest {
    let (kind, title, description) = match approval.kind {
        AiApprovalKind::CommandExecution => (
            "approval",
            "Command execution approval",
            approval
                .command
                .as_deref()
                .map(|command| format!("Command: {command}"))
                .or_else(|| {
                    approval
                        .cwd
                        .as_ref()
                        .map(|cwd| format!("Requested in {}", cwd.display()))
                })
                .unwrap_or_else(|| "Codex wants to run a command.".to_owned()),
        ),
        AiApprovalKind::FileChange => (
            "approval",
            "File change approval",
            approval
                .grant_root
                .as_ref()
                .map(|root| format!("Grant write access under {}", root.display()))
                .unwrap_or_else(|| "Codex wants to change files.".to_owned()),
        ),
    };
    AiPendingRequest {
        request_id: approval.request_id.clone(),
        thread_id: approval.thread_id.clone(),
        turn_id: approval.turn_id.clone(),
        kind: kind.to_owned(),
        title: bounded_text(title, MAX_HEADER_BYTES),
        description: bounded_text(description.as_str(), MAX_TEXT_BYTES),
        reason: bounded_text(
            approval.reason.as_deref().unwrap_or_default(),
            MAX_TEXT_BYTES,
        ),
        answerable: true,
        questions: Vec::new(),
    }
}

fn project_user_input(request: &AiPendingUserInputRequest) -> AiPendingRequest {
    let question_ids = request
        .questions
        .iter()
        .map(|question| question.id.as_str())
        .collect::<BTreeSet<_>>();
    let answerable = request.questions.len() <= MAX_QUESTIONS
        && question_ids.len() == request.questions.len()
        && request.questions.iter().all(|question| {
            question.options.len() <= MAX_OPTIONS
                && question
                    .options
                    .iter()
                    .all(|option| option.label.len() <= MAX_OPTION_BYTES)
        });
    AiPendingRequest {
        request_id: request.request_id.clone(),
        thread_id: request.thread_id.clone(),
        turn_id: request.turn_id.clone(),
        kind: "userInput".to_owned(),
        title: "Codex needs your input".to_owned(),
        description: "Answer the questions below so the active turn can continue.".to_owned(),
        reason: String::new(),
        answerable,
        questions: request
            .questions
            .iter()
            .take(MAX_QUESTIONS)
            .map(project_question)
            .collect(),
    }
}

fn project_question(question: &AiPendingUserInputQuestion) -> AiPendingQuestion {
    AiPendingQuestion {
        id: question.id.clone(),
        header: bounded_text(question.header.as_str(), MAX_HEADER_BYTES),
        question: bounded_text(question.question.as_str(), MAX_TEXT_BYTES),
        is_other: question.is_other,
        is_secret: question.is_secret,
        options: question
            .options
            .iter()
            .take(MAX_OPTIONS)
            .map(project_option)
            .collect(),
    }
}

fn project_option(option: &AiPendingUserInputQuestionOption) -> AiPendingOption {
    AiPendingOption {
        label: bounded_exact(option.label.as_str(), MAX_OPTION_BYTES),
        description: bounded_text(option.description.as_str(), MAX_OPTION_BYTES),
    }
}

fn bounded_exact(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes.saturating_sub('…'.len_utf8()).min(value.len());
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &value[..end])
}

fn bounded_text(value: &str, max_bytes: usize) -> String {
    let value = value.trim();
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes.saturating_sub('…'.len_utf8()).min(value.len());
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &value[..end])
}

fn saturating_usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
