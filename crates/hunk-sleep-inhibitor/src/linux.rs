use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use tracing::warn;

const ASSERTION_REASON: &str = "Hunk is running an AI turn";
const APP_ID: &str = "hunk";
const BLOCKER_SLEEP_SECONDS: &str = "2147483647";

#[derive(Debug, Default)]
pub(crate) struct SleepInhibitor {
    state: InhibitState,
    preferred_backend: Option<LinuxBackend>,
    missing_backend_logged: bool,
}

#[derive(Debug, Default)]
enum InhibitState {
    #[default]
    Inactive,
    Active {
        backend: LinuxBackend,
        child: Child,
    },
}

#[derive(Debug, Clone, Copy)]
enum LinuxBackend {
    SystemdInhibit,
    GnomeSessionInhibit,
}

impl SleepInhibitor {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn acquire(&mut self) {
        if let InhibitState::Active { backend, child } = &mut self.state {
            match child.try_wait() {
                Ok(None) => return,
                Ok(Some(status)) => warn!(
                    ?backend,
                    ?status,
                    "Linux sleep inhibitor backend exited unexpectedly; attempting fallback"
                ),
                Err(error) => warn!(
                    ?backend,
                    reason = %error,
                    "Failed to query Linux sleep inhibitor backend status; attempting restart"
                ),
            }
        }

        self.state = InhibitState::Inactive;
        let should_log_backend_failures = !self.missing_backend_logged;
        let backends = match self.preferred_backend {
            Some(LinuxBackend::SystemdInhibit) => [
                LinuxBackend::SystemdInhibit,
                LinuxBackend::GnomeSessionInhibit,
            ],
            Some(LinuxBackend::GnomeSessionInhibit) => [
                LinuxBackend::GnomeSessionInhibit,
                LinuxBackend::SystemdInhibit,
            ],
            None => [
                LinuxBackend::SystemdInhibit,
                LinuxBackend::GnomeSessionInhibit,
            ],
        };

        for backend in backends {
            match spawn_backend(backend) {
                Ok(mut child) => match child.try_wait() {
                    Ok(None) => {
                        self.state = InhibitState::Active { backend, child };
                        self.preferred_backend = Some(backend);
                        self.missing_backend_logged = false;
                        return;
                    }
                    Ok(Some(status)) => {
                        if should_log_backend_failures {
                            warn!(
                                ?backend,
                                ?status,
                                "Linux sleep inhibitor backend exited immediately"
                            );
                        }
                    }
                    Err(error) => {
                        if should_log_backend_failures {
                            warn!(
                                ?backend,
                                reason = %error,
                                "Failed to query Linux sleep inhibitor backend status after spawn"
                            );
                        }
                        if let Err(error) = child.kill()
                            && !child_exited(&error)
                        {
                            warn!(
                                ?backend,
                                reason = %error,
                                "Failed to stop Linux sleep inhibitor backend after status probe failure"
                            );
                        }
                        if let Err(error) = child.wait()
                            && !child_exited(&error)
                        {
                            warn!(
                                ?backend,
                                reason = %error,
                                "Failed to reap Linux sleep inhibitor backend after status probe failure"
                            );
                        }
                    }
                },
                Err(error) => {
                    if should_log_backend_failures && error.kind() != std::io::ErrorKind::NotFound {
                        warn!(
                            ?backend,
                            reason = %error,
                            "Failed to start Linux sleep inhibitor backend"
                        );
                    }
                }
            }
        }

        if should_log_backend_failures {
            warn!("No Linux sleep inhibitor backend is available");
            self.missing_backend_logged = true;
        }
    }

    pub(crate) fn release(&mut self) {
        match std::mem::take(&mut self.state) {
            InhibitState::Inactive => {}
            InhibitState::Active { backend, mut child } => {
                if let Err(error) = child.kill()
                    && !child_exited(&error)
                {
                    warn!(?backend, reason = %error, "Failed to stop Linux sleep inhibitor backend");
                }
                if let Err(error) = child.wait()
                    && !child_exited(&error)
                {
                    warn!(?backend, reason = %error, "Failed to reap Linux sleep inhibitor backend");
                }
            }
        }
    }
}

impl Drop for SleepInhibitor {
    fn drop(&mut self) {
        self.release();
    }
}

fn spawn_backend(backend: LinuxBackend) -> Result<Child, std::io::Error> {
    let parent_pid = unsafe { libc::getpid() };
    let mut command = match backend {
        LinuxBackend::SystemdInhibit => {
            let mut command = Command::new("systemd-inhibit");
            command.args([
                "--what=idle",
                "--mode=block",
                "--who",
                APP_ID,
                "--why",
                ASSERTION_REASON,
                "--",
                "sleep",
                BLOCKER_SLEEP_SECONDS,
            ]);
            command
        }
        LinuxBackend::GnomeSessionInhibit => {
            let mut command = Command::new("gnome-session-inhibit");
            command.args([
                "--inhibit",
                "idle",
                "--reason",
                ASSERTION_REASON,
                "sleep",
                BLOCKER_SLEEP_SECONDS,
            ]);
            command
        }
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() != parent_pid {
                libc::raise(libc::SIGTERM);
            }
            Ok(())
        });
    }

    command.spawn()
}

fn child_exited(error: &std::io::Error) -> bool {
    matches!(error.kind(), std::io::ErrorKind::InvalidInput)
}
