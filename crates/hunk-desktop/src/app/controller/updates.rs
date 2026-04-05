#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateCheckTrigger {
    Automatic,
    UserInitiated,
}

impl UpdateCheckTrigger {
    const fn should_notify_on_error(self) -> bool {
        matches!(self, Self::UserInitiated)
    }

    const fn should_notify_when_up_to_date(self) -> bool {
        matches!(self, Self::UserInitiated)
    }
}

impl DiffViewer {
    const AUTO_UPDATE_CHECK_INTERVAL_MS: i64 = 12 * 60 * 60 * 1000;

    pub(super) fn check_for_updates_action(
        &mut self,
        _: &CheckForUpdates,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_update_check(UpdateCheckTrigger::UserInitiated, Some(window), cx);
    }

    pub(super) fn maybe_schedule_startup_update_check(&mut self, cx: &mut Context<Self>) {
        if !self.config.auto_update_enabled {
            return;
        }
        if !matches!(self.update_install_source, InstallSource::SelfManaged) {
            return;
        }
        if matches!(self.update_status, UpdateStatus::Checking) {
            return;
        }

        let now = now_unix_ms();
        let due = self
            .config
            .last_update_check_at
            .is_none_or(|last_checked| now.saturating_sub(last_checked) >= Self::AUTO_UPDATE_CHECK_INTERVAL_MS);
        if !due {
            return;
        }

        self.start_update_check(UpdateCheckTrigger::Automatic, None, cx);
    }

    fn start_update_check(
        &mut self,
        trigger: UpdateCheckTrigger,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        if matches!(self.update_status, UpdateStatus::Checking) {
            if matches!(trigger, UpdateCheckTrigger::UserInitiated) {
                Self::push_warning_notification(
                    "An update check is already in progress.".to_string(),
                    window,
                    cx,
                );
            }
            return;
        }

        if let InstallSource::PackageManaged { explanation } = &self.update_install_source {
            self.update_status = UpdateStatus::DisabledByInstallSource {
                explanation: explanation.clone(),
            };
            self.git_status_message = Some(explanation.clone());
            if matches!(trigger, UpdateCheckTrigger::UserInitiated) {
                Self::push_warning_notification(explanation.clone(), window, cx);
            }
            cx.notify();
            return;
        }

        let manifest_url = hunk_updater::resolve_manifest_url();
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        let started_at = Instant::now();

        self.update_status = UpdateStatus::Checking;
        self.git_status_message = Some("Checking for updates...".to_string());
        cx.notify();

        self.update_check_task = cx.spawn(async move |this, cx| {
            let (manifest_url, result) = cx
                .background_executor()
                .spawn(async move {
                    let result = hunk_updater::check_for_updates(
                        manifest_url.as_str(),
                        current_version.as_str(),
                    );
                    (manifest_url, result)
                })
                .await;

            let checked_at = now_unix_ms();
            let total_elapsed = started_at.elapsed();
            let Some(this) = this.upgrade() else {
                return;
            };

            this.update(cx, |this, cx| {
                this.config.last_update_check_at = Some(checked_at);
                this.persist_config();

                match result {
                    Ok(hunk_updater::UpdateCheckResult::UpToDate { version }) => {
                        debug!(
                            manifest_url,
                            version,
                            elapsed_ms = total_elapsed.as_millis(),
                            "update check completed: up to date"
                        );
                        this.update_status = UpdateStatus::UpToDate {
                            version: version.clone(),
                            checked_at_unix_ms: checked_at,
                        };
                        this.git_status_message = Some(format!("Hunk is up to date ({version})."));
                        if trigger.should_notify_when_up_to_date() {
                            Self::push_success_notification(
                                format!("Hunk is up to date ({version})."),
                                cx,
                            );
                        }
                    }
                    Ok(hunk_updater::UpdateCheckResult::UpdateAvailable(update)) => {
                        let version = update.version.clone();
                        debug!(
                            manifest_url,
                            version,
                            elapsed_ms = total_elapsed.as_millis(),
                            "update check completed: update available"
                        );
                        this.update_status = UpdateStatus::UpdateAvailable(update);
                        let message = format!("Hunk {version} is available.");
                        this.git_status_message = Some(message.clone());
                        Self::push_warning_notification(message, None, cx);
                    }
                    Err(err) => {
                        error!(
                            manifest_url,
                            elapsed_ms = total_elapsed.as_millis(),
                            "update check failed: {err:#}"
                        );
                        let summary = err.to_string();
                        this.update_status = UpdateStatus::Error(summary.clone());
                        this.git_status_message = Some(format!("Update check failed: {summary}"));
                        if trigger.should_notify_on_error() {
                            Self::push_error_notification(
                                format!("Update check failed: {summary}"),
                                cx,
                            );
                        }
                    }
                }

                cx.notify();
            });
        });
    }
}
