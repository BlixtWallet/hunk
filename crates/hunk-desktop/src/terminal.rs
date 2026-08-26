use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use hunk_domain::config::{ConfigStore, TerminalConfig};
use hunk_terminal::{
    TerminalEvent, TerminalGridPoint, TerminalInputModifiers, TerminalKeystroke,
    TerminalMouseButton, TerminalScreenSnapshot, TerminalScroll, TerminalSessionHandle,
    TerminalSpawnRequest, resolve_terminal_shell, spawn_terminal_session, terminal_key_input,
    terminal_mouse_button_input, terminal_mouse_move_input, terminal_wheel_input,
};
use qtbridge::{QObjectHolder, invoke_method};

use crate::backend_state::Backend;
use crate::terminal_models::{
    TerminalScreenProjection, TerminalTabItem, project_terminal_screen, terminal_selection_text,
};

const TERMINAL_MAX_TABS: usize = 12;
const TERMINAL_MIN_ROWS: u16 = 2;
const TERMINAL_MAX_ROWS: u16 = 240;
const TERMINAL_MIN_COLS: u16 = 8;
const TERMINAL_MAX_COLS: u16 = 500;
const TERMINAL_MAX_COMMAND_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum TerminalStatus {
    #[default]
    Idle,
    Starting,
    Running,
    Completed,
    Failed,
}

impl TerminalStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

struct TerminalTabRuntime {
    id: i32,
    title: String,
    cwd: PathBuf,
    status: TerminalStatus,
    status_message: String,
    screen: Option<Arc<TerminalScreenSnapshot>>,
    projection: Option<TerminalScreenProjection>,
    handle: Option<TerminalSessionHandle>,
    generation: i32,
}

impl TerminalTabRuntime {
    fn idle(id: i32, title: String, cwd: PathBuf) -> Self {
        Self {
            id,
            title,
            cwd,
            status: TerminalStatus::Idle,
            status_message: String::new(),
            screen: None,
            projection: None,
            handle: None,
            generation: 0,
        }
    }

    fn item(&self) -> TerminalTabItem {
        TerminalTabItem {
            tab_id: self.id,
            title: self.title.clone(),
            status: self.status.as_str().to_owned(),
        }
    }
}

#[derive(Default)]
struct PendingTerminalEvent {
    screen: Option<PendingTerminalScreen>,
    end: Option<TerminalEnd>,
}

struct PendingTerminalScreen {
    snapshot: Arc<TerminalScreenSnapshot>,
    projection: TerminalScreenProjection,
}

enum TerminalEnd {
    Exit(Option<i32>),
    Failed(String),
}

#[derive(Default)]
struct TerminalEventMailbox {
    pending: HashMap<(i32, i32), PendingTerminalEvent>,
    wake_scheduled: bool,
}

impl TerminalEventMailbox {
    fn push(
        &mut self,
        tab_id: i32,
        generation: i32,
        update: PendingTerminalEvent,
        screen_visible: bool,
    ) -> bool {
        if update.screen.is_none() && update.end.is_none() {
            return false;
        }
        let pending = self.pending.entry((tab_id, generation)).or_default();
        if let Some(screen) = update.screen {
            pending.screen = Some(screen);
        }
        if let Some(end) = update.end {
            pending.end = Some(end);
        }
        let needs_ui_update = pending.end.is_some() || screen_visible;
        if !needs_ui_update {
            return false;
        }
        if self.wake_scheduled {
            false
        } else {
            self.wake_scheduled = true;
            true
        }
    }

    fn drain(&mut self) -> HashMap<(i32, i32), PendingTerminalEvent> {
        self.wake_scheduled = false;
        std::mem::take(&mut self.pending)
    }
}

pub(super) struct TerminalRuntimeState {
    root: PathBuf,
    config: TerminalConfig,
    shell_label: String,
    active_tab_id: i32,
    next_tab_id: i32,
    next_generation: i32,
    screen_dirty: bool,
    displayed_tab_id: Option<i32>,
    rows: u16,
    cols: u16,
    tabs: BTreeMap<i32, TerminalTabRuntime>,
    tab_order: Vec<i32>,
    mailbox: Arc<Mutex<TerminalEventMailbox>>,
    screen_visible: Arc<AtomicBool>,
    start_results: Arc<Mutex<HashMap<(i32, i32), TerminalStartResult>>>,
    start_tasks: Vec<JoinHandle<()>>,
    listeners: Vec<JoinHandle<()>>,
}

type TerminalStartResult = Result<StartedTerminal, String>;

struct StartedTerminal {
    handle: TerminalSessionHandle,
    event_rx: std::sync::mpsc::Receiver<TerminalEvent>,
}

impl Default for TerminalRuntimeState {
    fn default() -> Self {
        let config = TerminalConfig::default();
        Self {
            root: PathBuf::new(),
            shell_label: resolve_terminal_shell(&config).label().to_owned(),
            config,
            active_tab_id: 1,
            next_tab_id: 2,
            next_generation: 1,
            screen_dirty: false,
            displayed_tab_id: None,
            rows: 24,
            cols: 120,
            tabs: BTreeMap::new(),
            tab_order: vec![1],
            mailbox: Arc::new(Mutex::new(TerminalEventMailbox::default())),
            screen_visible: Arc::new(AtomicBool::new(false)),
            start_results: Arc::new(Mutex::new(HashMap::new())),
            start_tasks: Vec::new(),
            listeners: Vec::new(),
        }
    }
}

impl Drop for TerminalRuntimeState {
    fn drop(&mut self) {
        self.tabs.clear();
    }
}

pub(super) fn configure_terminal(backend: &mut Backend) {
    let config = ConfigStore::new()
        .and_then(|store| store.load_or_create_default())
        .map(|config| config.terminal);
    let warning = match config {
        Ok(config) => {
            backend.terminal_runtime.shell_label =
                resolve_terminal_shell(&config).label().to_owned();
            backend.terminal_runtime.config = config;
            None
        }
        Err(error) => Some(format!(
            "Using the system shell; terminal settings could not be loaded: {error:#}"
        )),
    };
    let root = PathBuf::from(backend.git_root.as_str());
    reset_terminal_root(backend, root, false);
    if let Some(warning) = warning {
        backend.terminal_status_message = warning;
        backend.terminal_state_changed();
    }
}

pub(super) fn reconcile_terminal_root(backend: &mut Backend) {
    let root = PathBuf::from(backend.git_root.as_str());
    if backend.terminal_runtime.root == root {
        return;
    }
    let reopen = backend.terminal_open;
    reset_terminal_root(backend, root, reopen);
}

pub(super) fn toggle_terminal(backend: &mut Backend) -> bool {
    set_terminal_open(backend, !backend.terminal_open)
}

pub(super) fn set_terminal_open(backend: &mut Backend, open: bool) -> bool {
    if backend.terminal_open == open {
        return false;
    }
    backend.terminal_open = open;
    backend
        .terminal_runtime
        .screen_visible
        .store(open, Ordering::Release);
    if open {
        drain_terminal_events(backend);
        request_terminal_focus(backend);
        ensure_active_terminal_started(backend);
    }
    sync_terminal_projection(backend);
    true
}

pub(super) fn new_terminal_tab(backend: &mut Backend) -> bool {
    if backend.terminal_runtime.tab_order.len() >= TERMINAL_MAX_TABS {
        backend.terminal_status_message =
            format!("A maximum of {TERMINAL_MAX_TABS} terminal tabs can be open.");
        backend.terminal_state_changed();
        return false;
    }
    let tab_id = backend.terminal_runtime.next_tab_id.max(1);
    backend.terminal_runtime.next_tab_id = tab_id.saturating_add(1);
    let tab = idle_terminal_tab(backend, tab_id, backend.terminal_runtime.root.clone());
    backend.terminal_runtime.tabs.insert(tab_id, tab);
    backend.terminal_runtime.tab_order.push(tab_id);
    backend.terminal_runtime.active_tab_id = tab_id;
    backend.terminal_open = true;
    backend
        .terminal_runtime
        .screen_visible
        .store(true, Ordering::Release);
    request_terminal_focus(backend);
    start_terminal_tab(backend, tab_id, None);
    sync_terminal_projection(backend);
    true
}

pub(super) fn select_terminal_tab(backend: &mut Backend, tab_id: i32) -> bool {
    if !backend.terminal_runtime.tabs.contains_key(&tab_id)
        || backend.terminal_runtime.active_tab_id == tab_id
    {
        return false;
    }
    backend.terminal_runtime.active_tab_id = tab_id;
    bump_terminal_screen_revision(backend);
    backend.terminal_open = true;
    backend
        .terminal_runtime
        .screen_visible
        .store(true, Ordering::Release);
    request_terminal_focus(backend);
    ensure_active_terminal_started(backend);
    resize_active_terminal(backend);
    sync_terminal_projection(backend);
    true
}

pub(super) fn close_terminal_tab(backend: &mut Backend, tab_id: i32) -> bool {
    let closing_active = backend.terminal_runtime.active_tab_id == tab_id;
    let Some(index) = backend
        .terminal_runtime
        .tab_order
        .iter()
        .position(|candidate| *candidate == tab_id)
    else {
        return false;
    };
    backend.terminal_runtime.tab_order.remove(index);
    backend.terminal_runtime.tabs.remove(&tab_id);

    if backend.terminal_runtime.tab_order.is_empty() {
        let replacement_id = backend.terminal_runtime.next_tab_id.max(1);
        backend.terminal_runtime.next_tab_id = replacement_id.saturating_add(1);
        let replacement = idle_terminal_tab(
            backend,
            replacement_id,
            backend.terminal_runtime.root.clone(),
        );
        backend
            .terminal_runtime
            .tabs
            .insert(replacement_id, replacement);
        backend.terminal_runtime.tab_order.push(replacement_id);
    }
    if backend.terminal_runtime.active_tab_id == tab_id {
        let replacement_index = index.min(backend.terminal_runtime.tab_order.len() - 1);
        backend.terminal_runtime.active_tab_id =
            backend.terminal_runtime.tab_order[replacement_index];
    }
    if closing_active {
        bump_terminal_screen_revision(backend);
    }
    if backend.terminal_open {
        request_terminal_focus(backend);
        ensure_active_terminal_started(backend);
    }
    sync_terminal_projection(backend);
    true
}

pub(super) fn move_terminal_tab(backend: &mut Backend, direction: i32) -> bool {
    let Some(index) = backend
        .terminal_runtime
        .tab_order
        .iter()
        .position(|tab_id| *tab_id == backend.terminal_runtime.active_tab_id)
    else {
        return false;
    };
    let len = backend.terminal_runtime.tab_order.len();
    if len < 2 {
        return false;
    }
    let target = if direction < 0 {
        (index + len - 1) % len
    } else {
        (index + 1) % len
    };
    let tab_id = backend.terminal_runtime.tab_order[target];
    select_terminal_tab(backend, tab_id)
}

pub(super) fn resize_terminal(backend: &mut Backend, rows: i32, cols: i32) -> bool {
    let rows = clamp_grid_dimension(rows, TERMINAL_MIN_ROWS, TERMINAL_MAX_ROWS);
    let cols = clamp_grid_dimension(cols, TERMINAL_MIN_COLS, TERMINAL_MAX_COLS);
    if backend.terminal_runtime.rows == rows && backend.terminal_runtime.cols == cols {
        return false;
    }
    backend.terminal_runtime.rows = rows;
    backend.terminal_runtime.cols = cols;
    resize_active_terminal(backend)
}

pub(super) fn send_terminal_key(
    backend: &mut Backend,
    key: String,
    text: String,
    shift: bool,
    control: bool,
    alt: bool,
    platform: bool,
) -> bool {
    let key_char = (!text.is_empty()).then_some(text.as_str());
    let Some(input) = terminal_key_input(&TerminalKeystroke {
        key: key.as_str(),
        key_char,
        modifiers: TerminalInputModifiers {
            shift,
            control,
            alt,
            platform,
            function: false,
        },
    }) else {
        return false;
    };
    active_terminal_handle(backend)
        .and_then(|handle| handle.write_key_input(input).ok())
        .is_some()
}

pub(super) fn write_terminal_text(backend: &mut Backend, text: String) -> bool {
    if text.is_empty() || text.len() > TERMINAL_MAX_COMMAND_BYTES || text.contains('\0') {
        return false;
    }
    active_terminal_handle(backend)
        .and_then(|handle| handle.write_input(text.as_bytes()).ok())
        .is_some()
}

pub(super) fn paste_terminal_text(backend: &mut Backend, text: String) -> bool {
    if text.is_empty() || text.len() > TERMINAL_MAX_COMMAND_BYTES || text.contains('\0') {
        return false;
    }
    active_terminal_handle(backend)
        .and_then(|handle| handle.write_paste(text.as_str()).ok())
        .is_some()
}

pub(super) fn report_terminal_focus(backend: &mut Backend, focused: bool) -> bool {
    active_terminal_handle(backend)
        .and_then(|handle| handle.report_focus(focused).ok())
        .is_some()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn send_terminal_pointer_button(
    backend: &mut Backend,
    row: i32,
    column: i32,
    button: i32,
    pressed: bool,
    shift: bool,
    control: bool,
    alt: bool,
) -> bool {
    let Some((point, mode)) = active_terminal_pointer_context(backend, row, column) else {
        return false;
    };
    let Some(button) = terminal_mouse_button(button) else {
        return false;
    };
    let Some(input) = terminal_mouse_button_input(
        point,
        button,
        TerminalInputModifiers {
            shift,
            control,
            alt,
            ..TerminalInputModifiers::default()
        },
        pressed,
        Some(mode),
    ) else {
        return false;
    };
    active_terminal_handle(backend)
        .and_then(|handle| handle.write_pointer_input(input).ok())
        .is_some()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn send_terminal_pointer_move(
    backend: &mut Backend,
    row: i32,
    column: i32,
    button: i32,
    shift: bool,
    control: bool,
    alt: bool,
) -> bool {
    let Some((point, mode)) = active_terminal_pointer_context(backend, row, column) else {
        return false;
    };
    let Some(input) = terminal_mouse_move_input(
        point,
        terminal_mouse_button(button),
        TerminalInputModifiers {
            shift,
            control,
            alt,
            ..TerminalInputModifiers::default()
        },
        Some(mode),
    ) else {
        return false;
    };
    active_terminal_handle(backend)
        .and_then(|handle| handle.write_pointer_input(input).ok())
        .is_some()
}

pub(super) fn send_terminal_wheel(
    backend: &mut Backend,
    row: i32,
    column: i32,
    lines: i32,
    shift: bool,
    control: bool,
    alt: bool,
) -> bool {
    if lines == 0 {
        return false;
    }
    let Some((point, mode)) = active_terminal_pointer_context(backend, row, column) else {
        return false;
    };
    let Some(input) = terminal_wheel_input(
        point,
        lines,
        TerminalInputModifiers {
            shift,
            control,
            alt,
            ..TerminalInputModifiers::default()
        },
    ) else {
        return false;
    };
    let fallback = (!mode.alt_screen).then_some(TerminalScroll::Delta(-lines));
    active_terminal_handle(backend)
        .and_then(|handle| handle.write_wheel_input(input, fallback).ok())
        .is_some()
}

pub(super) fn clear_terminal_screen(backend: &mut Backend) -> bool {
    active_terminal_handle(backend)
        .and_then(|handle| handle.write_input(b"\x0c").ok())
        .is_some()
}

pub(super) fn scroll_terminal(backend: &mut Backend, direction: String) -> bool {
    let Some(tab) = active_terminal_tab(backend) else {
        return false;
    };
    if tab
        .screen
        .as_deref()
        .is_some_and(|screen| screen.mode.alt_screen)
    {
        return false;
    }
    let scroll = match direction.as_str() {
        "pageUp" => TerminalScroll::PageUp,
        "pageDown" => TerminalScroll::PageDown,
        "top" => TerminalScroll::Top,
        "bottom" => TerminalScroll::Bottom,
        _ => return false,
    };
    active_terminal_handle(backend)
        .and_then(|handle| handle.scroll_display(scroll).ok())
        .is_some()
}

pub(super) fn selected_terminal_text(
    backend: &Backend,
    anchor_row: i32,
    anchor_column: i32,
    head_row: i32,
    head_column: i32,
) -> String {
    active_terminal_tab(backend)
        .and_then(|tab| tab.screen.as_deref())
        .map(|screen| {
            terminal_selection_text(screen, anchor_row, anchor_column, head_row, head_column)
        })
        .unwrap_or_default()
}

pub(super) fn run_terminal_command(backend: &mut Backend, command: String, cwd: String) -> bool {
    if command.trim().is_empty()
        || command.len() > TERMINAL_MAX_COMMAND_BYTES
        || command.contains('\0')
    {
        return false;
    }
    let requested_cwd = PathBuf::from(cwd);
    let cwd = if requested_cwd.as_os_str().is_empty() {
        backend.terminal_runtime.root.clone()
    } else {
        requested_cwd
    };
    backend.terminal_open = true;
    backend
        .terminal_runtime
        .screen_visible
        .store(true, Ordering::Release);
    request_terminal_focus(backend);

    let active_matches = active_terminal_tab(backend).is_some_and(|tab| {
        tab.status == TerminalStatus::Running && tab.cwd == cwd && tab.handle.is_some()
    });
    if !active_matches {
        if backend.terminal_runtime.tab_order.len() >= TERMINAL_MAX_TABS {
            backend.terminal_status_message =
                "Close a terminal tab before running another command.".to_owned();
            backend.terminal_state_changed();
            return false;
        }
        let tab_id = backend.terminal_runtime.next_tab_id.max(1);
        backend.terminal_runtime.next_tab_id = tab_id.saturating_add(1);
        let tab = idle_terminal_tab(backend, tab_id, cwd.clone());
        backend.terminal_runtime.tabs.insert(tab_id, tab);
        backend.terminal_runtime.tab_order.push(tab_id);
        backend.terminal_runtime.active_tab_id = tab_id;
        start_terminal_tab(backend, tab_id, Some(command));
    } else if let Some(handle) = active_terminal_handle(backend)
        && (handle.write_input(command.as_bytes()).is_err() || handle.write_input(b"\r").is_err())
    {
        sync_terminal_projection(backend);
        return false;
    }
    sync_terminal_projection(backend);
    true
}

pub(super) fn apply_terminal_events(backend: &mut Backend) {
    if drain_terminal_events(backend) {
        sync_terminal_projection(backend);
    }
}

fn drain_terminal_events(backend: &mut Backend) -> bool {
    let pending = backend
        .terminal_runtime
        .mailbox
        .lock()
        .map(|mut mailbox| mailbox.drain())
        .unwrap_or_default();
    if pending.is_empty() {
        return false;
    }

    let active_tab_id = backend.terminal_runtime.active_tab_id;
    let mut active_screen_changed = false;
    let mut metadata_changed = false;
    for ((tab_id, generation), event) in pending {
        let Some(tab) = backend.terminal_runtime.tabs.get_mut(&tab_id) else {
            continue;
        };
        if tab.generation != generation {
            continue;
        }
        if tab_id == active_tab_id && (event.screen.is_some() || event.end.is_some()) {
            active_screen_changed = true;
        }
        if let Some(screen) = event.screen {
            tab.screen = Some(screen.snapshot);
            tab.projection = Some(screen.projection);
        }
        if let Some(end) = event.end {
            metadata_changed = true;
            tab.handle = None;
            match end {
                TerminalEnd::Exit(exit_code) => {
                    tab.status = if exit_code.is_some_and(|code| code != 0) {
                        TerminalStatus::Failed
                    } else {
                        TerminalStatus::Completed
                    };
                    tab.status_message = exit_code
                        .map(|code| format!("Shell exited with status {code}."))
                        .unwrap_or_else(|| "Shell exited.".to_owned());
                }
                TerminalEnd::Failed(error) => {
                    tab.status = TerminalStatus::Failed;
                    tab.status_message = error;
                }
            }
        }
    }
    if active_screen_changed {
        bump_terminal_screen_revision(backend);
    }
    active_screen_changed || metadata_changed
}

fn reset_terminal_root(backend: &mut Backend, root: PathBuf, reopen: bool) {
    backend.terminal_runtime.tabs.clear();
    backend.terminal_runtime.root = root.clone();
    backend.terminal_runtime.active_tab_id = 1;
    backend.terminal_runtime.next_tab_id = 2;
    backend.terminal_runtime.displayed_tab_id = None;
    backend.terminal_runtime.tab_order.clear();
    backend.terminal_runtime.tab_order.push(1);
    let tab = idle_terminal_tab(backend, 1, root);
    backend.terminal_runtime.tabs.insert(1, tab);
    bump_terminal_screen_revision(backend);
    if reopen {
        start_terminal_tab(backend, 1, None);
    }
    sync_terminal_projection(backend);
}

fn idle_terminal_tab(backend: &Backend, tab_id: i32, cwd: PathBuf) -> TerminalTabRuntime {
    let title = if tab_id == 1 {
        backend.terminal_runtime.shell_label.clone()
    } else {
        format!("{} {tab_id}", backend.terminal_runtime.shell_label)
    };
    TerminalTabRuntime::idle(tab_id, title, cwd)
}

fn ensure_active_terminal_started(backend: &mut Backend) {
    let tab_id = backend.terminal_runtime.active_tab_id;
    let needs_start = backend
        .terminal_runtime
        .tabs
        .get(&tab_id)
        .is_some_and(|tab| tab.handle.is_none() && tab.status != TerminalStatus::Starting);
    if needs_start {
        start_terminal_tab(backend, tab_id, None);
    }
}

fn start_terminal_tab(backend: &mut Backend, tab_id: i32, command: Option<String>) {
    let Some(tab) = backend.terminal_runtime.tabs.get(&tab_id) else {
        return;
    };
    let cwd = tab.cwd.clone();
    let generation = backend.terminal_runtime.next_generation.max(1);
    backend.terminal_runtime.next_generation = generation.wrapping_add(1).max(1);
    let shell = resolve_terminal_shell(&backend.terminal_runtime.config);
    let rows = backend.terminal_runtime.rows;
    let cols = backend.terminal_runtime.cols;
    let request = TerminalSpawnRequest::shell(cwd.clone())
        .with_shell_program(shell.program().to_os_string())
        .with_shell_args(
            shell.interactive_shell_args(backend.terminal_runtime.config.inherit_login_environment),
        );
    if let Some(tab) = backend.terminal_runtime.tabs.get_mut(&tab_id) {
        tab.handle = None;
        tab.generation = generation;
        tab.status = TerminalStatus::Starting;
        tab.status_message = "Starting shell…".to_owned();
        tab.screen = None;
        tab.projection = None;
    }
    if tab_id == backend.terminal_runtime.active_tab_id {
        bump_terminal_screen_revision(backend);
    }

    backend
        .terminal_runtime
        .start_tasks
        .retain(|task| !task.is_finished());
    let invoker = backend.get_qml_method_invoker();
    let results = Arc::clone(&backend.terminal_runtime.start_results);
    let spawn_result = std::thread::Builder::new()
        .name(format!("hunk-desktop-terminal-start-{tab_id}"))
        .spawn(move || {
            let result = spawn_terminal_session(request)
                .map_err(|error| format!("Failed to start terminal shell: {error:#}"))
                .and_then(|(handle, event_rx)| {
                    handle
                        .resize(rows, cols)
                        .map_err(|error| format!("Failed to size terminal shell: {error:#}"))?;
                    if let Some(command) = command {
                        handle.write_input(command.as_bytes()).map_err(|error| {
                            format!("Failed to write terminal command: {error:#}")
                        })?;
                        handle.write_input(b"\r").map_err(|error| {
                            format!("Failed to submit terminal command: {error:#}")
                        })?;
                    }
                    Ok(StartedTerminal { handle, event_rx })
                });
            if let Ok(mut pending) = results.lock() {
                pending.insert((tab_id, generation), result);
            }
            invoke_method!(invoker, "complete_terminal_start", tab_id, generation);
        });
    match spawn_result {
        Ok(task) => backend.terminal_runtime.start_tasks.push(task),
        Err(error) => {
            if let Some(tab) = backend.terminal_runtime.tabs.get_mut(&tab_id) {
                tab.status = TerminalStatus::Failed;
                tab.status_message = format!("Failed to start terminal task: {error}");
            }
        }
    }
}

pub(super) fn complete_terminal_start(backend: &mut Backend, tab_id: i32, generation: i32) {
    backend
        .terminal_runtime
        .start_tasks
        .retain(|task| !task.is_finished());
    let result = backend
        .terminal_runtime
        .start_results
        .lock()
        .ok()
        .and_then(|mut pending| pending.remove(&(tab_id, generation)));
    let is_current = backend
        .terminal_runtime
        .tabs
        .get(&tab_id)
        .is_some_and(|tab| tab.generation == generation);
    if !is_current {
        return;
    }
    match result {
        Some(Ok(started)) => {
            if let Some(tab) = backend.terminal_runtime.tabs.get_mut(&tab_id) {
                tab.handle = Some(started.handle);
                tab.status = TerminalStatus::Running;
                tab.status_message.clear();
            }
            start_terminal_listener(backend, tab_id, generation, started.event_rx);
        }
        Some(Err(error)) => {
            if let Some(tab) = backend.terminal_runtime.tabs.get_mut(&tab_id) {
                tab.handle = None;
                tab.status = TerminalStatus::Failed;
                tab.status_message = error;
            }
        }
        None => {
            if let Some(tab) = backend.terminal_runtime.tabs.get_mut(&tab_id) {
                tab.handle = None;
                tab.status = TerminalStatus::Failed;
                tab.status_message = "Terminal start completed without a result.".to_owned();
            }
        }
    }
    sync_terminal_projection(backend);
}

fn start_terminal_listener(
    backend: &mut Backend,
    tab_id: i32,
    generation: i32,
    event_rx: std::sync::mpsc::Receiver<TerminalEvent>,
) {
    backend
        .terminal_runtime
        .listeners
        .retain(|listener| !listener.is_finished());
    let mailbox = Arc::clone(&backend.terminal_runtime.mailbox);
    let screen_visible = Arc::clone(&backend.terminal_runtime.screen_visible);
    let invoker = backend.get_qml_method_invoker();
    let listener = std::thread::Builder::new()
        .name(format!("hunk-desktop-terminal-{tab_id}"))
        .spawn(move || {
            while let Ok(event) = event_rx.recv() {
                let mut screen = None;
                let mut end = None;
                collect_terminal_event(&mut screen, &mut end, event);
                for event in event_rx.try_iter() {
                    collect_terminal_event(&mut screen, &mut end, event);
                }
                let update = PendingTerminalEvent {
                    screen: screen.map(|snapshot| PendingTerminalScreen {
                        projection: project_terminal_screen(snapshot.as_ref()),
                        snapshot,
                    }),
                    end,
                };
                let should_wake = mailbox
                    .lock()
                    .map(|mut mailbox| {
                        mailbox.push(
                            tab_id,
                            generation,
                            update,
                            screen_visible.load(Ordering::Acquire),
                        )
                    })
                    .unwrap_or(false);
                if should_wake {
                    invoke_method!(invoker, "apply_terminal_events", tab_id);
                }
            }
        });
    if let Ok(listener) = listener {
        backend.terminal_runtime.listeners.push(listener);
    } else if let Some(tab) = backend.terminal_runtime.tabs.get_mut(&tab_id) {
        tab.handle = None;
        tab.status = TerminalStatus::Failed;
        tab.status_message = "Failed to start the terminal event listener.".to_owned();
    }
}

fn collect_terminal_event(
    screen: &mut Option<Arc<TerminalScreenSnapshot>>,
    end: &mut Option<TerminalEnd>,
    event: TerminalEvent,
) {
    match event {
        TerminalEvent::Output(_) => {}
        TerminalEvent::Screen(snapshot) => *screen = Some(snapshot),
        TerminalEvent::Exit { exit_code } => {
            *end = Some(TerminalEnd::Exit(exit_code));
        }
        TerminalEvent::Failed(error) => {
            *end = Some(TerminalEnd::Failed(error));
        }
    }
}

fn resize_active_terminal(backend: &mut Backend) -> bool {
    let rows = backend.terminal_runtime.rows;
    let cols = backend.terminal_runtime.cols;
    active_terminal_handle(backend)
        .and_then(|handle| handle.resize(rows, cols).ok())
        .is_some()
}

fn active_terminal_pointer_context(
    backend: &Backend,
    row: i32,
    column: i32,
) -> Option<(TerminalGridPoint, hunk_terminal::TerminalModeSnapshot)> {
    let screen = active_terminal_tab(backend)?.screen.as_deref()?;
    let row = row.clamp(0, i32::from(screen.rows.saturating_sub(1)));
    let column = column.clamp(0, i32::from(screen.cols.saturating_sub(1)));
    Some((
        TerminalGridPoint {
            line: row.saturating_sub(screen.display_offset as i32),
            column: column as usize,
        },
        screen.mode,
    ))
}

fn terminal_mouse_button(button: i32) -> Option<TerminalMouseButton> {
    match button {
        1 => Some(TerminalMouseButton::Left),
        2 => Some(TerminalMouseButton::Right),
        4 => Some(TerminalMouseButton::Middle),
        _ => None,
    }
}

fn active_terminal_tab(backend: &Backend) -> Option<&TerminalTabRuntime> {
    backend
        .terminal_runtime
        .tabs
        .get(&backend.terminal_runtime.active_tab_id)
}

fn active_terminal_handle(backend: &Backend) -> Option<&TerminalSessionHandle> {
    active_terminal_tab(backend)?.handle.as_ref()
}

fn sync_terminal_projection(backend: &mut Backend) {
    let tab_items = backend
        .terminal_runtime
        .tab_order
        .iter()
        .filter_map(|tab_id| backend.terminal_runtime.tabs.get(tab_id))
        .map(TerminalTabRuntime::item)
        .collect();
    backend.terminal_tabs.borrow_mut().defer_replace(tab_items);
    backend.terminal_active_tab_id = backend.terminal_runtime.active_tab_id;
    backend.terminal_active_tab_index = backend
        .terminal_runtime
        .tab_order
        .iter()
        .position(|tab_id| *tab_id == backend.terminal_runtime.active_tab_id)
        .and_then(|index| i32::try_from(index).ok())
        .unwrap_or(-1);
    backend.terminal_shell_label = backend.terminal_runtime.shell_label.clone();

    let (status, status_message, cwd) = active_terminal_tab(backend)
        .map(|tab| {
            (
                tab.status.as_str().to_owned(),
                tab.status_message.clone(),
                tab.cwd.display().to_string(),
            )
        })
        .unwrap_or_else(|| {
            (
                TerminalStatus::Idle.as_str().to_owned(),
                String::new(),
                String::new(),
            )
        });
    backend.terminal_status = status;
    backend.terminal_status_message = status_message;
    backend.terminal_cwd = cwd;

    if backend.terminal_open {
        let active_tab_id = backend.terminal_runtime.active_tab_id;
        let new_rows = backend
            .terminal_runtime
            .tabs
            .get_mut(&active_tab_id)
            .and_then(|tab| tab.projection.as_mut())
            .and_then(|projection| {
                (!projection.rows.is_empty()).then(|| std::mem::take(&mut projection.rows))
            });
        if backend.terminal_runtime.displayed_tab_id != Some(active_tab_id) {
            let previous_tab_id = backend.terminal_runtime.displayed_tab_id;
            let previous_rows = backend
                .terminal_rows
                .borrow_mut()
                .defer_replace_for_tab(new_rows.unwrap_or_default());
            if let Some(previous_tab) =
                previous_tab_id.and_then(|tab_id| backend.terminal_runtime.tabs.get_mut(&tab_id))
                && let Some(projection) = previous_tab.projection.as_mut()
            {
                projection.rows = previous_rows;
            }
            backend.terminal_runtime.displayed_tab_id = Some(active_tab_id);
        } else if let Some(rows) = new_rows {
            backend
                .terminal_rows
                .borrow_mut()
                .defer_replace_or_patch(rows);
        }
    }

    let active_screen = active_terminal_tab(backend).and_then(|tab| {
        let screen = tab.screen.as_deref()?;
        let projection = tab.projection.as_ref()?;
        Some((
            i32::try_from(screen.display_offset).unwrap_or(i32::MAX),
            screen.mode.mouse_mode,
            projection.cursor_row,
            projection.cursor_column,
            projection.cursor_shape.clone(),
            projection.cursor_visible,
        ))
    });
    if let Some((
        display_offset,
        mouse_mode,
        cursor_row,
        cursor_column,
        cursor_shape,
        cursor_visible,
    )) = active_screen
    {
        backend.terminal_display_offset = display_offset;
        backend.terminal_mouse_mode = mouse_mode;
        backend.terminal_cursor_row = cursor_row;
        backend.terminal_cursor_column = cursor_column;
        backend.terminal_cursor_shape = cursor_shape;
        backend.terminal_cursor_visible = cursor_visible;
    } else {
        backend.terminal_rows.borrow_mut().defer_clear();
        backend.terminal_display_offset = 0;
        backend.terminal_mouse_mode = false;
        backend.terminal_cursor_row = -1;
        backend.terminal_cursor_column = -1;
        backend.terminal_cursor_shape = "hidden".to_owned();
        backend.terminal_cursor_visible = false;
    }
    backend.terminal_state_changed();
    if std::mem::take(&mut backend.terminal_runtime.screen_dirty) {
        backend.terminal_screen_changed();
    }
}

fn bump_terminal_screen_revision(backend: &mut Backend) {
    backend.terminal_screen_revision = backend.terminal_screen_revision.wrapping_add(1).max(1);
    backend.terminal_runtime.screen_dirty = true;
}

fn request_terminal_focus(backend: &mut Backend) {
    backend.terminal_focus_revision = backend.terminal_focus_revision.wrapping_add(1).max(1);
    backend.terminal_focus_changed();
}

fn clamp_grid_dimension(value: i32, minimum: u16, maximum: u16) -> u16 {
    u16::try_from(value)
        .unwrap_or(if value < 0 { minimum } else { maximum })
        .clamp(minimum, maximum)
}
