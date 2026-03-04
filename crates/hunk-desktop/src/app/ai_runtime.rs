use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use codex_app_server_protocol::CommandExecParams;
use codex_app_server_protocol::ReviewStartParams;
use codex_app_server_protocol::ReviewTarget;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::TurnInterruptParams;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnSteerParams;
use codex_app_server_protocol::UserInput;
use hunk_codex::api::InitializeOptions;
use hunk_codex::errors::CodexIntegrationError;
use hunk_codex::host::HostConfig;
use hunk_codex::host::HostRuntime;
use hunk_codex::state::AiState;
use hunk_codex::state::TurnStatus as StateTurnStatus;
use hunk_codex::threads::ThreadService;
use hunk_codex::ws_client::JsonRpcSession;
use hunk_codex::ws_client::WebSocketEndpoint;

const HOST_START_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const NOTIFICATION_POLL_TIMEOUT: Duration = Duration::from_millis(25);
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiConnectionState {
    Disconnected,
    Connecting,
    Ready,
    Failed,
}

#[derive(Debug, Clone)]
pub struct AiSnapshot {
    pub state: AiState,
    pub active_thread_id: Option<String>,
    pub last_command_result: Option<String>,
}

#[derive(Debug)]
pub enum AiWorkerEvent {
    Snapshot(AiSnapshot),
    Status(String),
    Error(String),
    Fatal(String),
}

#[derive(Debug)]
pub enum AiWorkerCommand {
    RefreshThreads,
    StartThread {
        prompt: Option<String>,
    },
    SelectThread {
        thread_id: String,
    },
    SendPrompt {
        thread_id: String,
        prompt: String,
    },
    InterruptTurn {
        thread_id: String,
        turn_id: String,
    },
    StartReview {
        thread_id: String,
        instructions: String,
    },
    CommandExec {
        command_line: String,
    },
}

#[derive(Debug, Clone)]
pub struct AiWorkerStartConfig {
    pub cwd: PathBuf,
    pub codex_executable: PathBuf,
    pub codex_home: PathBuf,
    pub request_timeout: Duration,
}

impl AiWorkerStartConfig {
    pub fn new(cwd: PathBuf, codex_executable: PathBuf, codex_home: PathBuf) -> Self {
        Self {
            cwd,
            codex_executable,
            codex_home,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
        }
    }
}

pub fn spawn_ai_worker(
    config: AiWorkerStartConfig,
    command_rx: Receiver<AiWorkerCommand>,
    event_tx: Sender<AiWorkerEvent>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        if let Err(error) = run_ai_worker(config, command_rx, &event_tx) {
            let _ = event_tx.send(AiWorkerEvent::Fatal(error.to_string()));
        }
    })
}

struct AiWorkerRuntime {
    host: HostRuntime,
    session: JsonRpcSession,
    service: ThreadService,
    cwd_key: String,
    request_timeout: Duration,
    last_command_result: Option<String>,
}

fn run_ai_worker(
    config: AiWorkerStartConfig,
    command_rx: Receiver<AiWorkerCommand>,
    event_tx: &Sender<AiWorkerEvent>,
) -> Result<(), CodexIntegrationError> {
    let mut runtime = AiWorkerRuntime::bootstrap(config)?;

    let _ = event_tx.send(AiWorkerEvent::Status(
        "Codex App Server connected over WebSocket".to_string(),
    ));
    runtime.refresh_thread_list(event_tx)?;

    loop {
        match command_rx.recv_timeout(POLL_INTERVAL) {
            Ok(command) => {
                if let Err(error) = runtime.handle_command(command, event_tx) {
                    let _ = event_tx.send(AiWorkerEvent::Error(error.to_string()));
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                runtime.poll_notifications(event_tx)?;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    let _ = runtime.host.stop();
    Ok(())
}

impl AiWorkerRuntime {
    fn bootstrap(config: AiWorkerStartConfig) -> Result<Self, CodexIntegrationError> {
        std::fs::create_dir_all(&config.codex_home)
            .map_err(CodexIntegrationError::HostProcessIo)?;

        let port = allocate_loopback_port()?;
        let cwd_key = config.cwd.to_string_lossy().to_string();
        let host_config = HostConfig::codex_app_server(
            config.codex_executable,
            config.cwd.clone(),
            config.codex_home,
            port,
        );
        let mut host = HostRuntime::new(host_config);
        host.start(HOST_START_TIMEOUT)?;

        let endpoint = WebSocketEndpoint::loopback(port);
        let mut session = JsonRpcSession::connect(&endpoint)?;
        session.initialize(InitializeOptions::default(), config.request_timeout)?;

        Ok(Self {
            host,
            session,
            service: ThreadService::new(config.cwd),
            cwd_key,
            request_timeout: config.request_timeout,
            last_command_result: None,
        })
    }

    fn handle_command(
        &mut self,
        command: AiWorkerCommand,
        event_tx: &Sender<AiWorkerEvent>,
    ) -> Result<(), CodexIntegrationError> {
        match command {
            AiWorkerCommand::RefreshThreads => {
                self.refresh_thread_list(event_tx)?;
            }
            AiWorkerCommand::StartThread { prompt } => {
                let response = self.service.start_thread(
                    &mut self.session,
                    ThreadStartParams::default(),
                    self.request_timeout,
                )?;
                self.service
                    .state_mut()
                    .set_active_thread_for_cwd(self.cwd_key.clone(), response.thread.id.clone());
                if let Some(prompt) = prompt {
                    self.send_prompt(response.thread.id, prompt)?;
                }
                self.emit_snapshot(event_tx);
            }
            AiWorkerCommand::SelectThread { thread_id } => {
                self.service
                    .state_mut()
                    .set_active_thread_for_cwd(self.cwd_key.clone(), thread_id);
                self.emit_snapshot(event_tx);
            }
            AiWorkerCommand::SendPrompt { thread_id, prompt } => {
                self.send_prompt(thread_id, prompt)?;
                self.emit_snapshot(event_tx);
            }
            AiWorkerCommand::InterruptTurn { thread_id, turn_id } => {
                self.service.interrupt_turn(
                    &mut self.session,
                    TurnInterruptParams { thread_id, turn_id },
                    self.request_timeout,
                )?;
                self.emit_snapshot(event_tx);
            }
            AiWorkerCommand::StartReview {
                thread_id,
                instructions,
            } => {
                self.service.start_review(
                    &mut self.session,
                    ReviewStartParams {
                        thread_id,
                        target: ReviewTarget::Custom { instructions },
                        delivery: None,
                    },
                    self.request_timeout,
                )?;
                self.emit_snapshot(event_tx);
            }
            AiWorkerCommand::CommandExec { command_line } => {
                let command = split_command_line(command_line.as_str());
                if command.is_empty() {
                    let _ =
                        event_tx.send(AiWorkerEvent::Error("Command cannot be empty".to_string()));
                    return Ok(());
                }

                let response = self.service.command_exec(
                    &mut self.session,
                    CommandExecParams {
                        command,
                        timeout_ms: None,
                        cwd: None,
                        sandbox_policy: None,
                    },
                    self.request_timeout,
                )?;
                let stderr = response.stderr.trim();
                let stdout = response.stdout.trim();
                self.last_command_result = Some(format!(
                    "exit {}\n{}{}",
                    response.exit_code,
                    stdout,
                    if stderr.is_empty() {
                        "".to_string()
                    } else if stdout.is_empty() {
                        stderr.to_string()
                    } else {
                        format!("\n{stderr}")
                    }
                ));
                self.emit_snapshot(event_tx);
            }
        }

        Ok(())
    }

    fn send_prompt(
        &mut self,
        thread_id: String,
        prompt: String,
    ) -> Result<(), CodexIntegrationError> {
        let trimmed = prompt.trim();
        if trimmed.is_empty() {
            return Ok(());
        }

        self.service
            .state_mut()
            .set_active_thread_for_cwd(self.cwd_key.clone(), thread_id.clone());

        if let Some(in_progress_turn_id) = self.in_progress_turn_id(thread_id.as_str()) {
            self.service.steer_turn(
                &mut self.session,
                TurnSteerParams {
                    thread_id,
                    input: vec![UserInput::Text {
                        text: trimmed.to_string(),
                        text_elements: Vec::new(),
                    }],
                    expected_turn_id: in_progress_turn_id,
                },
                self.request_timeout,
            )?;
            return Ok(());
        }

        self.service.start_turn(
            &mut self.session,
            TurnStartParams {
                thread_id,
                input: vec![UserInput::Text {
                    text: trimmed.to_string(),
                    text_elements: Vec::new(),
                }],
                ..TurnStartParams::default()
            },
            self.request_timeout,
        )?;
        Ok(())
    }

    fn in_progress_turn_id(&self, thread_id: &str) -> Option<String> {
        self.service
            .state()
            .turns
            .values()
            .filter(|turn| {
                turn.thread_id == thread_id && turn.status == StateTurnStatus::InProgress
            })
            .max_by_key(|turn| turn.last_sequence)
            .map(|turn| turn.id.clone())
    }

    fn refresh_thread_list(
        &mut self,
        event_tx: &Sender<AiWorkerEvent>,
    ) -> Result<(), CodexIntegrationError> {
        let response =
            self.service
                .list_threads(&mut self.session, None, Some(200), self.request_timeout)?;

        if self.service.active_thread_for_workspace().is_none()
            && let Some(first_thread) = response.data.first()
        {
            self.service
                .state_mut()
                .set_active_thread_for_cwd(self.cwd_key.clone(), first_thread.id.clone());
        }

        self.emit_snapshot(event_tx);
        Ok(())
    }

    fn poll_notifications(
        &mut self,
        event_tx: &Sender<AiWorkerEvent>,
    ) -> Result<(), CodexIntegrationError> {
        let captured = self
            .session
            .poll_server_notifications(NOTIFICATION_POLL_TIMEOUT)?;
        if captured == 0 {
            return Ok(());
        }

        self.service.apply_queued_notifications(&mut self.session);
        self.emit_snapshot(event_tx);
        Ok(())
    }

    fn emit_snapshot(&self, event_tx: &Sender<AiWorkerEvent>) {
        let _ = event_tx.send(AiWorkerEvent::Snapshot(AiSnapshot {
            state: self.service.state().clone(),
            active_thread_id: self
                .service
                .active_thread_for_workspace()
                .map(ToOwned::to_owned),
            last_command_result: self.last_command_result.clone(),
        }));
    }
}

fn split_command_line(raw: &str) -> Vec<String> {
    raw.split_whitespace().map(ToOwned::to_owned).collect()
}

fn allocate_loopback_port() -> Result<u16, CodexIntegrationError> {
    let listener =
        TcpListener::bind(("127.0.0.1", 0)).map_err(CodexIntegrationError::HostProcessIo)?;
    let port = listener
        .local_addr()
        .map_err(CodexIntegrationError::HostProcessIo)?
        .port();
    drop(listener);
    Ok(port)
}
