use std::time::{SystemTime, UNIX_EPOCH};

use hunk_codex::protocol::{Account, RateLimitSnapshot, RateLimitWindow};

const FIVE_HOURS_MINUTES: i64 = 300;
const SEVEN_DAYS_MINUTES: i64 = 10_080;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AiAccountProjection {
    pub summary: String,
    pub connected: bool,
    pub login_pending: bool,
    pub five_hour_limit: AiRateLimitWindowProjection,
    pub weekly_limit: AiRateLimitWindowProjection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AiRateLimitWindowProjection {
    pub available: bool,
    pub remaining_percent: i32,
    pub reset_label: String,
}

impl Default for AiAccountProjection {
    fn default() -> Self {
        Self {
            summary: "No account connected.".to_owned(),
            connected: false,
            login_pending: false,
            five_hour_limit: AiRateLimitWindowProjection::default(),
            weekly_limit: AiRateLimitWindowProjection::default(),
        }
    }
}

impl Default for AiRateLimitWindowProjection {
    fn default() -> Self {
        Self {
            available: false,
            remaining_percent: 0,
            reset_label: "Unavailable".to_owned(),
        }
    }
}

impl AiAccountProjection {
    pub fn from_snapshot(
        account: Option<&Account>,
        requires_openai_auth: bool,
        pending_login_id: Option<&str>,
        rate_limits: Option<&RateLimitSnapshot>,
    ) -> Self {
        Self::from_snapshot_at(
            account,
            requires_openai_auth,
            pending_login_id,
            rate_limits,
            unix_timestamp_now(),
        )
    }

    pub fn from_snapshot_at(
        account: Option<&Account>,
        requires_openai_auth: bool,
        pending_login_id: Option<&str>,
        rate_limits: Option<&RateLimitSnapshot>,
        now: i64,
    ) -> Self {
        let (five_hour, weekly) = rate_limits.map(rate_limit_windows).unwrap_or((None, None));
        Self {
            summary: account_summary(account, requires_openai_auth),
            connected: account.is_some(),
            login_pending: pending_login_id.is_some(),
            five_hour_limit: project_window(five_hour, now),
            weekly_limit: project_window(weekly, now),
        }
    }
}

fn account_summary(account: Option<&Account>, requires_openai_auth: bool) -> String {
    match account {
        Some(Account::ApiKey { .. }) => "Signed in with API key.".to_owned(),
        Some(Account::Chatgpt { email, plan_type }) => match email.as_deref() {
            Some(email) => format!("ChatGPT: {email} ({plan_type:?})"),
            None => format!("ChatGPT ({plan_type:?})"),
        },
        Some(Account::AmazonBedrock { .. }) => "Signed in with Amazon Bedrock.".to_owned(),
        None if requires_openai_auth => "Sign in with ChatGPT to run coding agents.".to_owned(),
        None => "No account connected.".to_owned(),
    }
}

fn rate_limit_windows(
    snapshot: &RateLimitSnapshot,
) -> (Option<&RateLimitWindow>, Option<&RateLimitWindow>) {
    let windows = [snapshot.primary.as_ref(), snapshot.secondary.as_ref()];
    let five_hour = windows
        .iter()
        .flatten()
        .find(|window| window.window_duration_mins == Some(FIVE_HOURS_MINUTES))
        .copied();
    let weekly = windows
        .iter()
        .flatten()
        .find(|window| window.window_duration_mins == Some(SEVEN_DAYS_MINUTES))
        .copied();
    if windows
        .iter()
        .flatten()
        .any(|window| window.window_duration_mins.is_some())
    {
        return (five_hour, weekly);
    }
    (snapshot.primary.as_ref(), snapshot.secondary.as_ref())
}

fn project_window(window: Option<&RateLimitWindow>, now: i64) -> AiRateLimitWindowProjection {
    let Some(window) = window else {
        return AiRateLimitWindowProjection::default();
    };
    AiRateLimitWindowProjection {
        available: true,
        remaining_percent: 100_i32.saturating_sub(window.used_percent.clamp(0, 100)),
        reset_label: window
            .resets_at
            .map(|reset_at| format_reset_label(reset_at, now))
            .unwrap_or_else(|| "Reset time unavailable".to_owned()),
    }
}

fn format_reset_label(reset_at: i64, now: i64) -> String {
    let remaining = reset_at.saturating_sub(now);
    if remaining <= 60 {
        return "Resets soon".to_owned();
    }
    let minutes = remaining / 60;
    if minutes < 60 {
        return format!("Resets in {minutes}m");
    }
    let hours = minutes / 60;
    if hours < 24 {
        let trailing_minutes = minutes % 60;
        return if trailing_minutes == 0 {
            format!("Resets in {hours}h")
        } else {
            format!("Resets in {hours}h {trailing_minutes}m")
        };
    }
    let days = hours / 24;
    let trailing_hours = hours % 24;
    if trailing_hours == 0 {
        format!("Resets in {days}d")
    } else {
        format!("Resets in {days}d {trailing_hours}h")
    }
}

fn unix_timestamp_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or_default()
}
