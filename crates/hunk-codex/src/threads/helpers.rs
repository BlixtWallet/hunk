fn lifecycle_status_from_thread_status(status: &ThreadStatus) -> ThreadLifecycleStatus {
    match status {
        ThreadStatus::Active { .. } => ThreadLifecycleStatus::Active,
        ThreadStatus::Idle | ThreadStatus::SystemError => ThreadLifecycleStatus::Idle,
        ThreadStatus::NotLoaded => ThreadLifecycleStatus::NotLoaded,
    }
}

fn thread_item_kind(item: &ThreadItem) -> &'static str {
    match item {
        ThreadItem::UserMessage { .. } => "userMessage",
        ThreadItem::AgentMessage { .. } => "agentMessage",
        ThreadItem::Plan { .. } => "plan",
        ThreadItem::Reasoning { .. } => "reasoning",
        ThreadItem::CommandExecution { .. } => "commandExecution",
        ThreadItem::FileChange { .. } => "fileChange",
        ThreadItem::McpToolCall { .. } => "mcpToolCall",
        ThreadItem::DynamicToolCall { .. } => "dynamicToolCall",
        ThreadItem::CollabAgentToolCall { .. } => "collabAgentToolCall",
        ThreadItem::WebSearch { .. } => "webSearch",
        ThreadItem::ImageView { .. } => "imageView",
        ThreadItem::EnteredReviewMode { .. } => "enteredReviewMode",
        ThreadItem::ExitedReviewMode { .. } => "exitedReviewMode",
        ThreadItem::ContextCompaction { .. } => "contextCompaction",
    }
}

fn thread_item_seed_content(item: &ThreadItem) -> Option<String> {
    match item {
        ThreadItem::UserMessage { content, .. } => user_message_seed_content(content.as_slice()),
        ThreadItem::AgentMessage { text, .. } | ThreadItem::Plan { text, .. } => {
            (!text.is_empty()).then(|| text.clone())
        }
        ThreadItem::Reasoning {
            summary, content, ..
        } => {
            let mut parts = String::new();
            if !summary.is_empty() {
                parts.push_str(&summary.join(""));
            }
            if !content.is_empty() {
                parts.push_str(&content.join(""));
            }
            (!parts.is_empty()).then_some(parts)
        }
        ThreadItem::CommandExecution {
            aggregated_output, ..
        } => aggregated_output.clone().filter(|value| !value.is_empty()),
        ThreadItem::FileChange { changes, .. } => {
            let joined = changes
                .iter()
                .map(|change| change.diff.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            (!joined.is_empty()).then_some(joined)
        }
        ThreadItem::McpToolCall { error, .. } => error.as_ref().map(|value| value.message.clone()),
        ThreadItem::EnteredReviewMode { review, .. }
        | ThreadItem::ExitedReviewMode { review, .. } => {
            (!review.is_empty()).then(|| review.clone())
        }
        ThreadItem::WebSearch { query, action, .. } => {
            let detail = web_search_detail(action.as_ref(), query.as_str());
            (!detail.is_empty()).then(|| format!("Searched {detail}"))
        }
        ThreadItem::DynamicToolCall { .. }
        | ThreadItem::CollabAgentToolCall { .. }
        | ThreadItem::ImageView { .. }
        | ThreadItem::ContextCompaction { .. } => None,
    }
}

fn web_search_action_detail(action: &codex_app_server_protocol::WebSearchAction) -> String {
    match action {
        codex_app_server_protocol::WebSearchAction::Search { query, queries } => {
            query.clone().filter(|value| !value.is_empty()).unwrap_or_else(|| {
                let first = queries
                    .as_ref()
                    .and_then(|items| items.first())
                    .cloned()
                    .unwrap_or_default();
                if queries.as_ref().is_some_and(|items| items.len() > 1) && !first.is_empty() {
                    format!("{first} ...")
                } else {
                    first
                }
            })
        }
        codex_app_server_protocol::WebSearchAction::OpenPage { url } => {
            url.clone().unwrap_or_default()
        }
        codex_app_server_protocol::WebSearchAction::FindInPage { url, pattern } => {
            match (pattern, url) {
                (Some(pattern), Some(url)) => format!("'{pattern}' in {url}"),
                (Some(pattern), None) => format!("'{pattern}'"),
                (None, Some(url)) => url.clone(),
                (None, None) => String::new(),
            }
        }
        codex_app_server_protocol::WebSearchAction::Other => String::new(),
    }
}

fn web_search_detail(
    action: Option<&codex_app_server_protocol::WebSearchAction>,
    query: &str,
) -> String {
    let detail = action.map(web_search_action_detail).unwrap_or_default();
    if detail.is_empty() {
        query.to_string()
    } else {
        detail
    }
}

fn user_message_seed_content(content: &[UserInput]) -> Option<String> {
    let text = content
        .iter()
        .filter_map(user_input_text_content)
        .collect::<Vec<_>>()
        .join("");
    let images = content
        .iter()
        .filter_map(user_input_local_image_name)
        .collect::<Vec<_>>();

    if text.is_empty() && images.is_empty() {
        return None;
    }

    if images.is_empty() {
        return Some(text);
    }

    let image_prefix = if images.len() == 1 {
        "[image] "
    } else {
        "[images] "
    };
    let image_summary = format!("{image_prefix}{}", images.join(", "));
    if text.is_empty() {
        Some(image_summary)
    } else {
        Some(format!("{text}\n{image_summary}"))
    }
}

fn user_input_text_content(input: &UserInput) -> Option<&str> {
    match input {
        UserInput::Text { text, .. } => Some(text.as_str()),
        _ => None,
    }
}

fn user_input_local_image_name(input: &UserInput) -> Option<String> {
    match input {
        UserInput::LocalImage { path } => Some(local_image_display_name(path.as_path())),
        _ => None,
    }
}

fn local_image_display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn thread_item_is_complete(item: &ThreadItem) -> bool {
    match item {
        ThreadItem::CommandExecution { status, .. } => {
            !matches!(status, CommandExecutionStatus::InProgress)
        }
        ThreadItem::FileChange { status, .. } => !matches!(status, PatchApplyStatus::InProgress),
        ThreadItem::McpToolCall { status, .. } => !matches!(status, McpToolCallStatus::InProgress),
        ThreadItem::DynamicToolCall { status, .. } => {
            !matches!(status, DynamicToolCallStatus::InProgress)
        }
        ThreadItem::CollabAgentToolCall { status, .. } => {
            !matches!(status, CollabAgentToolCallStatus::InProgress)
        }
        _ => false,
    }
}

fn request_id_key(request_id: &RequestId) -> String {
    match request_id {
        RequestId::Integer(value) => format!("int:{value}"),
        RequestId::String(value) => format!("str:{value}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_search_seed_content_prefers_action_query() {
        let item = ThreadItem::WebSearch {
            id: "ws_1".to_string(),
            query: "fallback".to_string(),
            action: Some(codex_app_server_protocol::WebSearchAction::Search {
                query: Some("weather: 30009".to_string()),
                queries: None,
            }),
        };

        assert_eq!(
            thread_item_seed_content(&item).as_deref(),
            Some("Searched weather: 30009")
        );
    }

    #[test]
    fn web_search_seed_content_uses_fallback_query_when_action_empty() {
        let item = ThreadItem::WebSearch {
            id: "ws_2".to_string(),
            query: "weather: New York, NY".to_string(),
            action: None,
        };

        assert_eq!(
            thread_item_seed_content(&item).as_deref(),
            Some("Searched weather: New York, NY")
        );
    }

    #[test]
    fn web_search_seed_content_formats_find_in_page() {
        let item = ThreadItem::WebSearch {
            id: "ws_3".to_string(),
            query: "fallback".to_string(),
            action: Some(codex_app_server_protocol::WebSearchAction::FindInPage {
                pattern: Some("rain".to_string()),
                url: Some("https://example.com/weather".to_string()),
            }),
        };

        assert_eq!(
            thread_item_seed_content(&item).as_deref(),
            Some("Searched 'rain' in https://example.com/weather")
        );
    }
}
