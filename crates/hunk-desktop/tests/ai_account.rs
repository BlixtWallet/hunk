use hunk_codex::protocol::{Account, RateLimitSnapshot, RateLimitWindow};
use hunk_desktop::AiAccountProjection;

fn rate_limits(primary: RateLimitWindow, secondary: RateLimitWindow) -> RateLimitSnapshot {
    RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        primary: Some(primary),
        secondary: Some(secondary),
        credits: None,
        individual_limit: None,
        spend_control_reached: None,
        plan_type: None,
        rate_limit_reached_type: None,
    }
}

#[test]
fn account_projection_describes_authentication_and_login_state() {
    let required = AiAccountProjection::from_snapshot_at(None, true, Some("login"), None, 1_000);
    assert_eq!(
        required.summary,
        "Sign in with ChatGPT to run coding agents."
    );
    assert!(!required.connected);
    assert!(required.login_pending);

    let api_key =
        AiAccountProjection::from_snapshot_at(Some(&Account::ApiKey {}), false, None, None, 1_000);
    assert_eq!(api_key.summary, "Signed in with API key.");
    assert!(api_key.connected);
    assert!(!api_key.login_pending);
}

#[test]
fn account_projection_selects_named_windows_and_formats_resets() {
    let snapshot = rate_limits(
        RateLimitWindow {
            used_percent: 80,
            window_duration_mins: Some(10_080),
            resets_at: Some(1_000 + 4 * 24 * 60 * 60),
        },
        RateLimitWindow {
            used_percent: 28,
            window_duration_mins: Some(300),
            resets_at: Some(1_000 + 2 * 60 * 60),
        },
    );

    let projection =
        AiAccountProjection::from_snapshot_at(None, false, None, Some(&snapshot), 1_000);
    assert!(projection.five_hour_limit.available);
    assert_eq!(projection.five_hour_limit.remaining_percent, 72);
    assert_eq!(projection.five_hour_limit.reset_label, "Resets in 2h");
    assert!(projection.weekly_limit.available);
    assert_eq!(projection.weekly_limit.remaining_percent, 20);
    assert_eq!(projection.weekly_limit.reset_label, "Resets in 4d");
}

#[test]
fn account_projection_clamps_usage_and_handles_missing_reset_time() {
    let snapshot = rate_limits(
        RateLimitWindow {
            used_percent: 140,
            window_duration_mins: Some(300),
            resets_at: None,
        },
        RateLimitWindow {
            used_percent: -10,
            window_duration_mins: Some(10_080),
            resets_at: Some(1_020),
        },
    );

    let projection =
        AiAccountProjection::from_snapshot_at(None, false, None, Some(&snapshot), 1_000);
    assert_eq!(projection.five_hour_limit.remaining_percent, 0);
    assert_eq!(
        projection.five_hour_limit.reset_label,
        "Reset time unavailable"
    );
    assert_eq!(projection.weekly_limit.remaining_percent, 100);
    assert_eq!(projection.weekly_limit.reset_label, "Resets soon");
}

#[test]
fn account_projection_does_not_duplicate_a_named_weekly_window() {
    let snapshot = RateLimitSnapshot {
        limit_id: None,
        limit_name: None,
        primary: Some(RateLimitWindow {
            used_percent: 41,
            window_duration_mins: Some(10_080),
            resets_at: Some(1_000 + 6 * 24 * 60 * 60),
        }),
        secondary: None,
        credits: None,
        individual_limit: None,
        spend_control_reached: None,
        plan_type: None,
        rate_limit_reached_type: None,
    };

    let projection =
        AiAccountProjection::from_snapshot_at(None, false, None, Some(&snapshot), 1_000);
    assert!(!projection.five_hour_limit.available);
    assert!(projection.weekly_limit.available);
    assert_eq!(projection.weekly_limit.remaining_percent, 59);
}
