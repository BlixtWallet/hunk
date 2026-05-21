//! Cross-platform helper for preventing idle sleep while AI work is running.
//!
//! Adapted from OpenAI Codex's `codex-utils-sleep-inhibitor` crate:
//! https://github.com/openai/codex/tree/main/codex-rs/utils/sleep-inhibitor
//!
//! Platform behavior:
//! - macOS: native IOKit power assertions.
//! - Linux: `systemd-inhibit`, falling back to `gnome-session-inhibit`.
//! - Windows: `PowerCreateRequest` + `PowerSetRequest`.
//! - Other platforms: no-op backend.

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod dummy;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use dummy as platform;
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(target_os = "windows")]
use windows as platform;

#[derive(Debug)]
pub struct SleepInhibitor {
    enabled: bool,
    turn_running: bool,
    platform: platform::SleepInhibitor,
}

impl SleepInhibitor {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            turn_running: false,
            platform: platform::SleepInhibitor::new(),
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        self.sync();
    }

    pub fn set_turn_running(&mut self, turn_running: bool) {
        self.turn_running = turn_running;
        self.sync();
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_turn_running(&self) -> bool {
        self.turn_running
    }

    fn sync(&mut self) {
        if self.enabled && self.turn_running {
            self.platform.acquire();
        } else {
            self.platform.release();
        }
    }
}
