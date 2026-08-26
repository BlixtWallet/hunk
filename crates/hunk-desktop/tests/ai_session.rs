use std::collections::BTreeMap;

use hunk_codex::protocol::{InputModality, Model, ReasoningEffort, ReasoningEffortOption};
use hunk_codex::state::{AiState, ThreadTokenUsageSummary, TokenUsageBreakdownSummary};
use hunk_desktop::{
    AiContextUsageProjection, AiSessionCatalogProjection, AiSessionChoiceItem, AiSessionPreferences,
};
use hunk_domain::state::{
    AiCollaborationModeSelection, AiServiceTierSelection, AiThreadSessionState, AppState,
};

fn session(
    model: &str,
    effort: &str,
    collaboration_mode: AiCollaborationModeSelection,
    service_tier: AiServiceTierSelection,
) -> AiThreadSessionState {
    AiThreadSessionState {
        model: Some(model.to_owned()),
        effort: Some(effort.to_owned()),
        collaboration_mode,
        service_tier: Some(service_tier),
    }
}

fn choice(value: &str, label: &str) -> AiSessionChoiceItem {
    AiSessionChoiceItem {
        value: value.to_owned(),
        label: label.to_owned(),
        description: String::new(),
        hidden: false,
        is_default: false,
    }
}

fn model() -> Model {
    Model {
        id: "gpt-test".to_owned(),
        model: "gpt-test".to_owned(),
        upgrade: None,
        upgrade_info: None,
        availability_nux: None,
        display_name: "GPT Test".to_owned(),
        description: "Model exposed to the Qt session picker.".to_owned(),
        model_specialty: None,
        hidden: false,
        supported_reasoning_efforts: vec![
            ReasoningEffortOption {
                reasoning_effort: ReasoningEffort::Low,
                description: "Faster responses".to_owned(),
            },
            ReasoningEffortOption {
                reasoning_effort: ReasoningEffort::High,
                description: "Deeper reasoning".to_owned(),
            },
        ],
        default_reasoning_effort: ReasoningEffort::High,
        input_modalities: vec![InputModality::Text, InputModality::Image],
        supports_personality: false,
        multi_agent_version: None,
        additional_speed_tiers: Vec::new(),
        service_tiers: Vec::new(),
        default_service_tier: None,
        is_default: true,
    }
}

#[test]
fn preferences_resolve_thread_then_workspace_then_product_defaults() {
    let mut state = AppState::default();
    state.ai_workspace_session_overrides.insert(
        "/repo".to_owned(),
        session(
            "gpt-workspace",
            "medium",
            AiCollaborationModeSelection::Default,
            AiServiceTierSelection::Standard,
        ),
    );
    state.ai_thread_session_overrides.insert(
        "thread-1".to_owned(),
        session(
            "gpt-thread",
            "high",
            AiCollaborationModeSelection::Plan,
            AiServiceTierSelection::Fast,
        ),
    );

    let preferences = AiSessionPreferences::from_state(&state);
    assert_eq!(
        preferences.resolved_session(Some("thread-1"), Some("/repo")),
        state.ai_thread_session_overrides["thread-1"]
    );
    assert_eq!(
        preferences.resolved_session(Some("missing"), Some("/repo")),
        state.ai_workspace_session_overrides["/repo"]
    );
    assert_eq!(
        preferences.resolved_session(None, Some("/other")),
        AiThreadSessionState::preferred_defaults()
    );
    assert!(preferences.workspace_mad_max("/repo"));
    assert!(preferences.workspace_include_hidden_models("/repo"));
}

#[test]
fn preferences_round_trip_only_session_owned_app_state_fields() {
    let mut source = AppState::default();
    source
        .ai_workspace_mad_max
        .insert("/repo".to_owned(), false);
    source
        .ai_workspace_include_hidden_models
        .insert("/repo".to_owned(), false);
    source.ai_workspace_session_overrides.insert(
        "/repo".to_owned(),
        session(
            "gpt-test",
            "high",
            AiCollaborationModeSelection::Plan,
            AiServiceTierSelection::Flex,
        ),
    );
    let preferences = AiSessionPreferences::from_state(&source);

    let mut target = AppState {
        ai_bookmarked_thread_ids: ["keep-me".to_owned()].into_iter().collect(),
        git_workflow_cache_by_repo: BTreeMap::new(),
        ..AppState::default()
    };
    preferences.apply_to_state(&mut target);

    assert_eq!(target.ai_workspace_mad_max, source.ai_workspace_mad_max);
    assert_eq!(
        target.ai_workspace_include_hidden_models,
        source.ai_workspace_include_hidden_models
    );
    assert_eq!(
        target.ai_workspace_session_overrides,
        source.ai_workspace_session_overrides
    );
    assert!(target.ai_bookmarked_thread_ids.contains("keep-me"));
}

#[test]
fn session_catalog_bounds_models_and_normalizes_model_effort_pairs() {
    let projection = AiSessionCatalogProjection::from_snapshot(
        &AiState::default(),
        None,
        &[model()],
        true,
        true,
    );

    assert_eq!(projection.models[0].label, "Server default");
    assert_eq!(projection.models[1].value, "gpt-test");
    assert_eq!(projection.efforts_by_model["gpt-test"].len(), 3);
    assert_eq!(projection.efforts_by_model["gpt-test"][2].label, "High");
    assert!(projection.model_supports_image_inputs(Some("gpt-test")));
    assert!(projection.model_supports_image_inputs(None));

    let selected = projection.normalized_session(session(
        "gpt-test",
        "high",
        AiCollaborationModeSelection::Plan,
        AiServiceTierSelection::Fast,
    ));
    assert_eq!(selected.model.as_deref(), Some("gpt-test"));
    assert_eq!(selected.effort.as_deref(), Some("high"));

    let unsupported_effort = projection.normalized_session(session(
        "gpt-test",
        "ultra",
        AiCollaborationModeSelection::Default,
        AiServiceTierSelection::Standard,
    ));
    assert_eq!(unsupported_effort.model.as_deref(), Some("gpt-test"));
    assert_eq!(unsupported_effort.effort, None);

    let unavailable_model = projection.normalized_session(session(
        "removed-model",
        "high",
        AiCollaborationModeSelection::Default,
        AiServiceTierSelection::Standard,
    ));
    assert_eq!(unavailable_model.model, None);
    assert_eq!(unavailable_model.effort, None);
}

#[test]
fn session_catalog_rejects_images_for_text_only_models() {
    let mut model = model();
    model.input_modalities = vec![InputModality::Text];
    let projection =
        AiSessionCatalogProjection::from_snapshot(&AiState::default(), None, &[model], true, true);

    assert!(!projection.model_supports_image_inputs(Some("gpt-test")));
    assert!(!projection.model_supports_image_inputs(None));
}

#[test]
fn session_catalog_bounds_runtime_labels_without_splitting_unicode() {
    let mut model = model();
    model.display_name = "界".repeat(80);

    let projection =
        AiSessionCatalogProjection::from_snapshot(&AiState::default(), None, &[model], true, true);

    assert!(projection.models[1].label.len() <= 128);
    assert_eq!(projection.models[1].label.chars().count(), 42);
}

#[test]
fn context_projection_matches_codex_baseline_adjusted_window_math() {
    let usage = ThreadTokenUsageSummary {
        turn_id: "turn-1".to_owned(),
        total: TokenUsageBreakdownSummary::default(),
        last: TokenUsageBreakdownSummary {
            total_tokens: 72_000,
            input_tokens: 42_000,
            cached_input_tokens: 12_000,
            output_tokens: 8_000,
            reasoning_output_tokens: 3_000,
        },
        model_context_window: Some(132_000),
        last_sequence: 1,
    };

    let projection = AiContextUsageProjection::from_usage(Some(&usage));
    assert!(projection.available);
    assert_eq!(projection.percent_used, 50);
    assert_eq!(projection.percent_left, 50);
    assert_eq!(projection.context_tokens, 72_000);
    assert_eq!(projection.input_tokens, 30_000);
    assert_eq!(projection.cached_input_tokens, 12_000);
    assert_eq!(projection.billable_tokens, 38_000);

    let unavailable = AiContextUsageProjection::from_usage(None);
    assert!(!unavailable.available);
}

#[test]
fn catalog_normalization_preserves_non_model_session_choices() {
    let projection = AiSessionCatalogProjection {
        models: vec![choice("", "Server default"), choice("gpt-test", "GPT Test")],
        efforts_by_model: [(
            "gpt-test".to_owned(),
            vec![choice("", "Model default"), choice("high", "High")],
        )]
        .into_iter()
        .collect(),
        ..AiSessionCatalogProjection::default()
    };
    let selected = projection.normalized_session(session(
        "missing",
        "high",
        AiCollaborationModeSelection::Plan,
        AiServiceTierSelection::Flex,
    ));

    assert_eq!(selected.model, None);
    assert_eq!(selected.effort, None);
    assert_eq!(
        selected.collaboration_mode,
        AiCollaborationModeSelection::Plan
    );
    assert_eq!(selected.service_tier, Some(AiServiceTierSelection::Flex));
}
