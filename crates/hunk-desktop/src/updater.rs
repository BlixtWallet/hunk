use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use hunk_domain::config::ConfigStore;
use hunk_updater::{InstallSource, StagedUpdate, UpdateCheckResult};
use qtbridge::{QObjectHolder, invoke_method, qobject, qtbridge_type_lib::QString};

const AUTOMATIC_UPDATE_CHECK_INTERVAL_MS: i64 = 10 * 60 * 1_000;

type UpdateTaskResult = Result<UpdateTaskOutcome, String>;

enum UpdateTaskOutcome {
    UpToDate { version: String },
    Ready(StagedUpdate),
}

pub struct UpdateBridge {
    enabled: bool,
    busy: bool,
    ready_to_restart: bool,
    status: String,
    status_message: String,
    version: String,
    bootstrapped: bool,
    installing: bool,
    epoch: i32,
    results: Arc<Mutex<HashMap<i32, UpdateTaskResult>>>,
    ready_update: Option<StagedUpdate>,
    shutdown_requested: Arc<AtomicBool>,
}

impl Default for UpdateBridge {
    fn default() -> Self {
        Self {
            enabled: true,
            busy: false,
            ready_to_restart: false,
            status: "idle".to_owned(),
            status_message: String::new(),
            version: String::new(),
            bootstrapped: false,
            installing: false,
            epoch: 0,
            results: Arc::new(Mutex::new(HashMap::new())),
            ready_update: None,
            shutdown_requested: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[qobject]
impl UpdateBridge {
    qproperty!("enabled", Member = enabled, Notify = state_changed);
    qproperty!("busy", Member = busy, Notify = state_changed);
    qproperty!(
        "readyToRestart",
        Member = ready_to_restart,
        Notify = state_changed
    );
    qproperty!("status", Member = status, Notify = state_changed);
    qproperty!(
        "statusMessage",
        Member = status_message,
        Notify = state_changed
    );
    qproperty!("version", Member = version, Notify = state_changed);

    #[qsignal]
    fn state_changed(&mut self);

    #[qsignal]
    fn quit_requested(&mut self);

    #[qslot]
    fn bootstrap(&mut self) {
        if self.bootstrapped {
            return;
        }
        self.bootstrapped = true;

        if let InstallSource::PackageManaged { explanation } = hunk_updater::detect_install_source()
        {
            self.enabled = false;
            self.status = "disabled".to_owned();
            self.status_message = explanation;
            self.state_changed();
            return;
        }

        match load_update_config() {
            Ok((auto_update_enabled, last_update_check_at))
                if auto_update_enabled && update_check_due(last_update_check_at, now_unix_ms()) =>
            {
                self.start_update_check();
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(%error, "failed to load updater configuration");
            }
        }
    }

    #[qslot]
    fn check_for_updates(&mut self) -> bool {
        self.start_update_check()
    }

    #[qslot]
    fn poll(&mut self) -> bool {
        if !self.enabled || self.busy || self.ready_to_restart {
            return false;
        }
        let Ok((true, last_update_check_at)) = load_update_config() else {
            return false;
        };
        if !update_check_due(last_update_check_at, now_unix_ms()) {
            return false;
        }
        self.start_update_check()
    }

    #[qslot]
    fn mark_update_downloading(&mut self, epoch: i32, version: String) {
        if self.epoch != epoch || !self.busy {
            return;
        }
        self.status = "downloading".to_owned();
        self.status_message = format!("Downloading Hunk {version}…");
        self.version = version;
        self.state_changed();
    }

    #[qslot]
    fn complete_update_check(&mut self, epoch: i32) {
        if self.epoch != epoch {
            return;
        }
        let result = self
            .results
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&epoch)
            .unwrap_or_else(|| Err("Updater worker returned no result.".to_owned()));

        self.busy = false;
        match result {
            Ok(UpdateTaskOutcome::UpToDate { version }) => {
                self.status = "up_to_date".to_owned();
                self.status_message = format!("Hunk is up to date ({version}).");
                self.version = version;
            }
            Ok(UpdateTaskOutcome::Ready(staged_update)) => {
                self.cleanup_ready_update();
                self.version.clone_from(&staged_update.version);
                self.status = "ready".to_owned();
                self.status_message = format!("Hunk {} is ready to install.", self.version);
                self.ready_to_restart = true;
                self.ready_update = Some(staged_update);
            }
            Err(error) => {
                self.status = "error".to_owned();
                self.status_message = format!("Update failed: {error}");
            }
        }
        self.state_changed();
    }

    #[qslot]
    fn restart_to_update(&mut self) -> bool {
        if !self.enabled || self.busy || !self.ready_to_restart {
            return false;
        }
        let Some(staged_update) = self.ready_update.as_ref() else {
            return false;
        };

        match crate::updater_helper::spawn_staged_update_apply(staged_update) {
            Ok(()) => {
                self.installing = true;
                self.busy = true;
                self.ready_to_restart = false;
                self.status = "installing".to_owned();
                self.status_message = format!("Installing Hunk {}…", staged_update.version);
                self.state_changed();
                self.quit_requested();
                true
            }
            Err(error) => {
                self.status = "error".to_owned();
                self.status_message = format!("Update install failed: {error:#}");
                self.state_changed();
                false
            }
        }
    }

    #[qslot]
    fn shutdown(&mut self) {
        self.shutdown_requested.store(true, Ordering::Release);
        if !self.installing {
            self.cleanup_ready_update();
            let pending = std::mem::take(
                &mut *self
                    .results
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()),
            );
            for result in pending.into_values() {
                cleanup_task_result(result);
            }
        }
    }
}

impl UpdateBridge {
    fn start_update_check(&mut self) -> bool {
        if !self.enabled || self.busy || self.ready_to_restart {
            return false;
        }

        self.epoch = self.epoch.wrapping_add(1).max(1);
        let epoch = self.epoch;
        let manifest_url = hunk_updater::resolve_manifest_url();
        let current_version = env!("CARGO_PKG_VERSION").to_owned();
        let results = Arc::clone(&self.results);
        let shutdown_requested = Arc::clone(&self.shutdown_requested);
        let invoker = self.get_qml_method_invoker();

        self.busy = true;
        self.status = "checking".to_owned();
        self.status_message = "Checking for Hunk updates…".to_owned();
        self.state_changed();

        let spawn_result = std::thread::Builder::new()
            .name("hunk-updater-check".to_owned())
            .spawn(move || {
                let result = run_update_check(
                    manifest_url.as_str(),
                    current_version.as_str(),
                    epoch,
                    &invoker,
                );
                if shutdown_requested.load(Ordering::Acquire) {
                    cleanup_task_result(result);
                    return;
                }
                results
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .insert(epoch, result);
                if shutdown_requested.load(Ordering::Acquire) {
                    let result = results
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .remove(&epoch);
                    if let Some(result) = result {
                        cleanup_task_result(result);
                    }
                    return;
                }
                if !invoke_method!(invoker, "complete_update_check", epoch) {
                    let result = results
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .remove(&epoch);
                    if let Some(result) = result {
                        cleanup_task_result(result);
                    }
                }
            });

        if let Err(error) = spawn_result {
            self.busy = false;
            self.status = "error".to_owned();
            self.status_message = format!("Failed to start updater: {error}");
            self.state_changed();
            return false;
        }
        true
    }

    fn cleanup_ready_update(&mut self) {
        if let Some(staged_update) = self.ready_update.take() {
            cleanup_staged_update(&staged_update);
        }
        self.ready_to_restart = false;
    }
}

fn run_update_check(
    manifest_url: &str,
    current_version: &str,
    epoch: i32,
    invoker: &qtbridge::QmlMethodInvoker,
) -> UpdateTaskResult {
    match hunk_updater::check_for_updates(manifest_url, current_version)
        .map_err(|error| format!("{error:#}"))?
    {
        UpdateCheckResult::UpToDate { version } => {
            record_successful_update_check();
            Ok(UpdateTaskOutcome::UpToDate { version })
        }
        UpdateCheckResult::UpdateAvailable(update) => {
            record_successful_update_check();
            invoke_method!(
                invoker,
                "mark_update_downloading",
                epoch,
                QString::from(update.version.clone())
            );
            let public_key =
                hunk_updater::required_public_key_base64().map_err(|error| format!("{error:#}"))?;
            hunk_updater::stage_available_update(&update, public_key.as_str())
                .map(UpdateTaskOutcome::Ready)
                .map_err(|error| format!("{error:#}"))
        }
    }
}

fn load_update_config() -> anyhow::Result<(bool, Option<i64>)> {
    let config = ConfigStore::new()?.load_or_create_default()?;
    Ok((config.auto_update_enabled, config.last_update_check_at))
}

fn record_successful_update_check() {
    let result = (|| -> anyhow::Result<()> {
        let store = ConfigStore::new()?;
        let mut config = store.load_or_create_default()?;
        config.last_update_check_at = Some(now_unix_ms());
        store.save(&config)
    })();
    if let Err(error) = result {
        tracing::warn!(%error, "failed to persist updater check timestamp");
    }
}

fn update_check_due(last_update_check_at: Option<i64>, now: i64) -> bool {
    last_update_check_at.is_none_or(|last_checked| {
        now.saturating_sub(last_checked) >= AUTOMATIC_UPDATE_CHECK_INTERVAL_MS
    })
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn cleanup_task_result(result: UpdateTaskResult) {
    if let Ok(UpdateTaskOutcome::Ready(staged_update)) = result {
        cleanup_staged_update(&staged_update);
    }
}

fn cleanup_staged_update(staged_update: &StagedUpdate) {
    let parent = staged_update.package_path.parent().map(ToOwned::to_owned);
    let _ = std::fs::remove_file(staged_update.package_path.as_path());
    if let Some(parent) = parent {
        let _ = std::fs::remove_dir(parent);
    }
}
