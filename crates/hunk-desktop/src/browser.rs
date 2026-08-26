use std::cell::RefCell;
#[cfg(all(feature = "cef-browser", target_os = "macos"))]
use std::path::Path;
#[cfg(feature = "cef-browser")]
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::Sender;

use hunk_app::ai::{
    BrowserToolSafetyMode, browser_confirmation_declined_response,
    browser_dynamic_tool_confirmation, browser_unavailable_response,
    execute_browser_dynamic_tool_with_runtime_and_safety,
};
use hunk_browser::{
    BrowserAction, BrowserContextMenuTarget, BrowserFrame, BrowserInputModifiers,
    BrowserMouseButton, BrowserMouseInput, BrowserPhysicalPoint, BrowserRuntime,
    BrowserRuntimeStatus, BrowserTabId, BrowserViewportSize,
};
#[cfg(feature = "cef-browser")]
use hunk_browser::{BrowserRuntimeConfig, BrowserStoragePaths};
use hunk_codex::protocol::{DynamicToolCallParams, DynamicToolCallResponse};
use qtbridge::{QObjectHolder, invoke_method, qobject};

use crate::browser_frame::{clear_browser_frame, publish_browser_frame};
use crate::browser_models::{
    BrowserTabListModel, BrowserTabProjectionSource, browser_tab_projection_changed,
    project_browser_tab_sources, project_browser_tabs,
};

struct PendingBrowserApproval {
    params: DynamicToolCallParams,
    response_tx: Sender<DynamicToolCallResponse>,
}

pub struct BrowserBridge {
    tabs: Rc<RefCell<BrowserTabListModel>>,
    runtime: BrowserRuntime,
    active_thread_id: String,
    active_tab_id: String,
    active_tab_index: i32,
    url: String,
    title: String,
    runtime_status: String,
    status_message: String,
    loading: bool,
    can_go_back: bool,
    can_go_forward: bool,
    open: bool,
    pump_active: bool,
    projected_tab_sources: Vec<BrowserTabProjectionSource>,
    frame_source: Option<(String, String, u64)>,
    presentation_epoch: u64,
    approval_pending: bool,
    approval_kind: String,
    approval_summary: String,
    pending_approval: Option<PendingBrowserApproval>,
    context_target: Option<BrowserContextMenuTarget>,
    context_target_json: String,
    context_clipboard_text: String,
}

impl Default for BrowserBridge {
    fn default() -> Self {
        Self {
            tabs: BrowserTabListModel::default_with_attached_qobject(),
            runtime: BrowserRuntime::new_disabled(),
            active_thread_id: String::new(),
            active_tab_id: String::new(),
            active_tab_index: -1,
            url: String::new(),
            title: String::new(),
            runtime_status: "idle".to_owned(),
            status_message: String::new(),
            loading: false,
            can_go_back: false,
            can_go_forward: false,
            open: false,
            pump_active: false,
            projected_tab_sources: Vec::new(),
            frame_source: None,
            presentation_epoch: 0,
            approval_pending: false,
            approval_kind: String::new(),
            approval_summary: String::new(),
            pending_approval: None,
            context_target: None,
            context_target_json: String::new(),
            context_clipboard_text: String::new(),
        }
    }
}

impl BrowserBridge {
    fn shutdown_runtime(&mut self) {
        if let Some(pending) = self.pending_approval.take() {
            let response = browser_unavailable_response(
                &pending.params,
                "The embedded browser closed before the action was confirmed.",
            );
            let _ = pending.response_tx.send(response);
        }
        self.runtime.shutdown_backend();
    }

    fn notify_state_changed(&self) {
        let invoker = self.get_qml_method_invoker();
        invoke_method!(invoker, "state_changed");
    }

    fn notify_approval_changed(&self) {
        let invoker = self.get_qml_method_invoker();
        invoke_method!(invoker, "approval_changed");
    }

    fn notify_context_changed(&self) {
        let invoker = self.get_qml_method_invoker();
        invoke_method!(invoker, "context_changed");
    }

    fn notify_context_menu_requested(&self, x: i32, y: i32) {
        let invoker = self.get_qml_method_invoker();
        invoke_method!(invoker, "context_menu_requested", x, y);
    }
}

#[qobject]
impl BrowserBridge {
    qproperty!("tabs", Member = tabs, Constant);
    qproperty!(
        "activeThreadId",
        Member = active_thread_id,
        Notify = state_changed
    );
    qproperty!(
        "activeTabId",
        Member = active_tab_id,
        Notify = state_changed
    );
    qproperty!(
        "activeTabIndex",
        Member = active_tab_index,
        Notify = state_changed
    );
    qproperty!("url", Member = url, Notify = state_changed);
    qproperty!("title", Member = title, Notify = state_changed);
    qproperty!(
        "runtimeStatus",
        Member = runtime_status,
        Notify = state_changed
    );
    qproperty!(
        "statusMessage",
        Member = status_message,
        Notify = state_changed
    );
    qproperty!("loading", Member = loading, Notify = state_changed);
    qproperty!("canGoBack", Member = can_go_back, Notify = state_changed);
    qproperty!(
        "canGoForward",
        Member = can_go_forward,
        Notify = state_changed
    );
    qproperty!("open", Member = open, Notify = state_changed);
    qproperty!("pumpActive", Member = pump_active, Notify = state_changed);
    qproperty!(
        "approvalPending",
        Member = approval_pending,
        Notify = approval_changed
    );
    qproperty!(
        "approvalKind",
        Member = approval_kind,
        Notify = approval_changed
    );
    qproperty!(
        "approvalSummary",
        Member = approval_summary,
        Notify = approval_changed
    );
    qproperty!(
        "contextTargetJson",
        Member = context_target_json,
        Notify = context_changed
    );
    qproperty!(
        "contextClipboardText",
        Member = context_clipboard_text,
        Notify = context_changed
    );

    #[qsignal]
    fn state_changed(&mut self);

    #[qsignal]
    fn approval_changed(&mut self);

    #[qsignal]
    fn context_changed(&mut self);

    #[qsignal]
    fn context_menu_requested(&mut self, x: i32, y: i32);

    #[qslot]
    fn set_open(&mut self, open: bool) -> bool {
        if !open {
            if !self.open {
                return false;
            }
            self.open = false;
            self.pump_active = false;
            let thread_id = self.active_thread_id.clone();
            if !thread_id.is_empty() && self.runtime.status() == BrowserRuntimeStatus::Ready {
                let _ = self
                    .runtime
                    .focus_backend_session(thread_id.as_str(), false);
            }
            self.notify_state_changed();
            return true;
        }

        let thread_id = self.active_thread_id.clone();
        if thread_id.is_empty() {
            self.set_status("Select a Codex thread before opening the browser.");
            return false;
        }
        if let Err(error) = self.ensure_backend_session(thread_id.as_str()) {
            self.set_status(error);
            return false;
        }
        self.open = true;
        self.sync_projection();
        self.notify_state_changed();
        true
    }

    #[qslot]
    fn navigate(&mut self, address: String) -> bool {
        let Some(url) = normalize_browser_address(address.as_str()) else {
            return false;
        };
        self.apply_visible_action(BrowserAction::Navigate { url })
    }

    #[qslot]
    fn go_back(&mut self) -> bool {
        self.apply_visible_action(BrowserAction::Back)
    }

    #[qslot]
    fn go_forward(&mut self) -> bool {
        self.apply_visible_action(BrowserAction::Forward)
    }

    #[qslot]
    fn reload(&mut self) -> bool {
        self.apply_visible_action(BrowserAction::Reload)
    }

    #[qslot]
    fn stop(&mut self) -> bool {
        self.apply_visible_action(BrowserAction::Stop)
    }

    #[qslot]
    fn new_tab(&mut self) -> bool {
        let Some(thread_id) = self.visible_thread_id() else {
            return false;
        };
        if let Err(error) = self.ensure_backend_session(thread_id.as_str()) {
            self.set_status(error);
            return false;
        }
        let tab_id = self.runtime.create_tab(thread_id.as_str(), None, true);
        if let Err(error) = self.runtime.ensure_backend_session(thread_id.clone()) {
            let _ = self.runtime.close_tab(thread_id.as_str(), &tab_id);
            self.set_status(error.to_string());
            return false;
        }
        self.sync_projection();
        true
    }

    #[qslot]
    fn select_tab(&mut self, tab_id: String) -> bool {
        let Some(thread_id) = self.visible_thread_id() else {
            return false;
        };
        if let Err(error) = self
            .runtime
            .select_tab(thread_id.as_str(), &BrowserTabId::new(tab_id))
            .and_then(|()| self.runtime.ensure_backend_session(thread_id.clone()))
        {
            self.set_status(error.to_string());
            return false;
        }
        self.sync_projection();
        true
    }

    #[qslot]
    fn close_tab(&mut self, tab_id: String) -> bool {
        let Some(thread_id) = self.visible_thread_id() else {
            return false;
        };
        if let Err(error) = self
            .runtime
            .close_tab(thread_id.as_str(), &BrowserTabId::new(tab_id))
            .and_then(|()| self.runtime.ensure_backend_session(thread_id.clone()))
        {
            self.set_status(error.to_string());
            return false;
        }
        self.sync_projection();
        true
    }

    #[qslot]
    fn toggle_devtools(&mut self) -> bool {
        let Some(thread_id) = self.visible_thread_id() else {
            return false;
        };
        let has_devtools = self
            .runtime
            .has_devtools(thread_id.as_str())
            .unwrap_or(false);
        let result = if has_devtools {
            self.runtime.close_devtools(thread_id.as_str())
        } else {
            self.runtime.show_devtools(thread_id.as_str(), None)
        };
        if let Err(error) = result {
            self.set_status(error.to_string());
            return false;
        }
        true
    }

    #[qslot]
    fn resize(&mut self, width: i32, height: i32, scale: f32) -> bool {
        let Some(thread_id) = self.visible_thread_id() else {
            return false;
        };
        let Ok(viewport) =
            BrowserViewportSize::new(width.max(1) as u32, height.max(1) as u32, scale)
        else {
            return false;
        };
        if let Err(error) = self
            .runtime
            .resize_backend_session(thread_id.as_str(), viewport)
        {
            self.set_status(error.to_string());
            return false;
        }
        true
    }

    #[qslot]
    fn republish_frame(&mut self) -> bool {
        let Some(thread_id) = self.visible_thread_id() else {
            return false;
        };
        let Some(frame) = self
            .runtime
            .session(thread_id.as_str())
            .and_then(|session| session.latest_frame())
            .cloned()
        else {
            return false;
        };
        self.publish_frame(&frame);
        true
    }

    #[qslot]
    fn report_focus(&mut self, focused: bool) -> bool {
        let Some(thread_id) = self.visible_thread_id() else {
            return false;
        };
        if let Err(error) = self
            .runtime
            .focus_backend_session(thread_id.as_str(), focused)
        {
            self.set_status(error.to_string());
            return false;
        }
        true
    }

    #[qslot]
    fn mouse_move(
        &mut self,
        x: i32,
        y: i32,
        shift: bool,
        control: bool,
        alt: bool,
        meta: bool,
    ) -> bool {
        let Some(thread_id) = self.visible_thread_id() else {
            return false;
        };
        let input = browser_mouse_input(x, y, shift, control, alt, meta);
        self.runtime
            .send_backend_mouse_move(thread_id.as_str(), input)
            .map(|()| true)
            .unwrap_or_else(|error| {
                self.set_status(error.to_string());
                false
            })
    }

    #[qslot]
    #[allow(clippy::too_many_arguments)] // QtBridge exposes pointer fields as individual QML values.
    fn mouse_click(
        &mut self,
        x: i32,
        y: i32,
        button: String,
        shift: bool,
        control: bool,
        alt: bool,
        meta: bool,
    ) -> bool {
        let Some(thread_id) = self.visible_thread_id() else {
            return false;
        };
        let button = match button.as_str() {
            "middle" => BrowserMouseButton::Middle,
            "right" => BrowserMouseButton::Right,
            _ => BrowserMouseButton::Left,
        };
        let input = browser_mouse_input(x, y, shift, control, alt, meta);
        self.runtime
            .send_backend_mouse_click(thread_id.as_str(), input, button)
            .map(|()| true)
            .unwrap_or_else(|error| {
                self.set_status(error.to_string());
                false
            })
    }

    #[qslot]
    #[allow(clippy::too_many_arguments)] // QtBridge exposes wheel fields as individual QML values.
    fn wheel(
        &mut self,
        x: i32,
        y: i32,
        delta_x: i32,
        delta_y: i32,
        shift: bool,
        control: bool,
        alt: bool,
        meta: bool,
    ) -> bool {
        let Some(thread_id) = self.visible_thread_id() else {
            return false;
        };
        let input = browser_mouse_input(x, y, shift, control, alt, meta);
        self.runtime
            .send_backend_mouse_wheel(thread_id.as_str(), input, delta_x, delta_y)
            .map(|()| true)
            .unwrap_or_else(|error| {
                self.set_status(error.to_string());
                false
            })
    }

    #[qslot]
    fn key_press(&mut self, keys: String) -> bool {
        let Some(thread_id) = self.visible_thread_id() else {
            return false;
        };
        self.runtime
            .send_backend_key_press(thread_id.as_str(), keys.as_str())
            .map(|()| true)
            .unwrap_or_else(|error| {
                self.set_status(error.to_string());
                false
            })
    }

    #[qslot]
    fn text_input(&mut self, text: String) -> bool {
        let Some(thread_id) = self.visible_thread_id() else {
            return false;
        };
        if text.is_empty() {
            return false;
        }
        self.runtime
            .send_backend_text(thread_id.as_str(), text.as_str())
            .map(|()| true)
            .unwrap_or_else(|error| {
                self.set_status(error.to_string());
                false
            })
    }

    #[qslot]
    fn pump(&mut self, request_frame: bool) -> bool {
        if !self.pump_active || self.runtime.status() != BrowserRuntimeStatus::Ready {
            return false;
        }
        match self.runtime.pump_backend_with_frame_request(request_frame) {
            Ok(changed) => {
                self.sync_context_menu();
                if changed {
                    self.sync_projection();
                }
                changed
            }
            Err(error) => {
                self.pump_active = false;
                self.runtime_status = "failed".to_owned();
                self.set_status(error.to_string());
                false
            }
        }
    }

    #[qslot]
    fn resolve_approval(&mut self, accept: bool) -> bool {
        let Some(pending) = self.pending_approval.take() else {
            return false;
        };
        let response = if accept {
            self.execute_tool_call(&pending.params, BrowserToolSafetyMode::AllowSensitiveOnce)
        } else {
            browser_confirmation_declined_response(&pending.params)
        };
        let _ = pending.response_tx.send(response);
        self.approval_pending = false;
        self.approval_kind.clear();
        self.approval_summary.clear();
        self.notify_approval_changed();
        true
    }

    #[qslot]
    fn context_action(&mut self, action: String) -> bool {
        self.run_context_action(action.as_str())
    }

    #[qslot]
    fn shutdown(&mut self) {
        self.shutdown_runtime();
    }
}

impl BrowserBridge {
    pub fn set_active_thread(&mut self, thread_id: String) {
        if self.active_thread_id == thread_id {
            return;
        }
        self.active_thread_id = thread_id;
        self.context_target = None;
        self.context_target_json.clear();
        self.notify_context_changed();
        self.notify_state_changed();
        if self.active_thread_id.is_empty() {
            self.open = false;
            self.pump_active = false;
            self.clear_projection();
            return;
        }
        if self
            .runtime
            .session(self.active_thread_id.as_str())
            .is_some()
        {
            let _ = self
                .runtime
                .set_visible_session(self.active_thread_id.clone());
        }
        if self.open {
            let thread_id = self.active_thread_id.clone();
            if let Err(error) = self.ensure_backend_session(thread_id.as_str()) {
                self.set_status(error);
                return;
            }
        }
        self.sync_projection();
    }

    pub fn handle_ai_tool_call(
        &mut self,
        params: DynamicToolCallParams,
        response_tx: Sender<DynamicToolCallResponse>,
    ) {
        if let Some(confirmation) = browser_dynamic_tool_confirmation(&params) {
            if let Some(previous) = self.pending_approval.take() {
                let response = browser_unavailable_response(
                    &previous.params,
                    "Another browser action replaced this confirmation request.",
                );
                let _ = previous.response_tx.send(response);
            }
            self.active_thread_id = params.thread_id.clone();
            self.open = true;
            self.approval_pending = true;
            self.approval_kind = sensitive_action_label(confirmation.kind);
            self.approval_summary = confirmation.summary;
            self.pending_approval = Some(PendingBrowserApproval {
                params,
                response_tx,
            });
            self.notify_approval_changed();
            self.notify_state_changed();
            return;
        }

        let response = self.execute_tool_call(&params, BrowserToolSafetyMode::Enforce);
        let _ = response_tx.send(response);
    }

    fn execute_tool_call(
        &mut self,
        params: &DynamicToolCallParams,
        safety_mode: BrowserToolSafetyMode,
    ) -> DynamicToolCallResponse {
        if let Err(error) = self.ensure_backend_session(params.thread_id.as_str()) {
            self.set_status(error.clone());
            return browser_unavailable_response(params, error.as_str());
        }
        self.active_thread_id = params.thread_id.clone();
        self.open = true;
        let response = execute_browser_dynamic_tool_with_runtime_and_safety(
            &mut self.runtime,
            params,
            true,
            safety_mode,
        );
        self.sync_projection();
        self.notify_state_changed();
        response
    }

    fn ensure_backend_session(&mut self, thread_id: &str) -> Result<(), String> {
        self.ensure_runtime_ready()?;
        self.runtime
            .ensure_backend_session(thread_id.to_owned())
            .map_err(|error| error.to_string())?;
        self.pump_active = true;
        Ok(())
    }

    fn ensure_runtime_ready(&mut self) -> Result<(), String> {
        if self.runtime.status() == BrowserRuntimeStatus::Ready {
            return Ok(());
        }

        #[cfg(not(feature = "cef-browser"))]
        {
            self.runtime_status = "unavailable".to_owned();
            Err("Hunk was built without the cef-browser feature.".to_owned())
        }

        #[cfg(feature = "cef-browser")]
        {
            if self.runtime.status() == BrowserRuntimeStatus::Disabled {
                self.runtime_status = "starting".to_owned();
                self.notify_state_changed();
                self.runtime = BrowserRuntime::new_configured(browser_runtime_config()?);
            }
            self.runtime.initialize_backend().map_err(|error| {
                self.runtime_status = "failed".to_owned();
                error.to_string()
            })?;
            self.runtime_status = "ready".to_owned();
            self.status_message.clear();
            Ok(())
        }
    }

    fn apply_visible_action(&mut self, action: BrowserAction) -> bool {
        let Some(thread_id) = self.visible_thread_id() else {
            return false;
        };
        if let Err(error) = self.ensure_backend_session(thread_id.as_str()) {
            self.set_status(error);
            return false;
        }
        if let Err(error) = self
            .runtime
            .apply_backend_action(thread_id.as_str(), &action)
        {
            self.set_status(error.to_string());
            return false;
        }
        self.sync_projection();
        true
    }

    fn visible_thread_id(&self) -> Option<String> {
        (!self.active_thread_id.is_empty()).then(|| self.active_thread_id.clone())
    }

    fn sync_projection(&mut self) {
        let Some(thread_id) = self.visible_thread_id() else {
            self.clear_projection();
            return;
        };
        let Some(session) = self.runtime.session(thread_id.as_str()) else {
            self.clear_projection();
            return;
        };
        let state = session.state();
        let active_tab_id = state.active_tab_id.as_str().to_owned();
        let active_tab_index = state
            .tabs
            .iter()
            .position(|tab| tab.tab_id == state.active_tab_id)
            .and_then(|index| i32::try_from(index).ok())
            .unwrap_or(-1);
        let url = state.url.clone().unwrap_or_default();
        let title = state.title.clone().unwrap_or_default();
        let status_message = state.load_error.clone().unwrap_or_default();
        let scalar_changed = self.active_tab_id != active_tab_id
            || self.active_tab_index != active_tab_index
            || self.url != url
            || self.title != title
            || self.loading != state.loading
            || self.can_go_back != state.can_go_back
            || self.can_go_forward != state.can_go_forward
            || self.status_message != status_message;
        let frame = session.latest_frame().cloned();

        self.active_tab_id = active_tab_id;
        self.active_tab_index = active_tab_index;
        self.url = url;
        self.title = title;
        self.loading = state.loading;
        self.can_go_back = state.can_go_back;
        self.can_go_forward = state.can_go_forward;
        self.status_message = status_message;
        if browser_tab_projection_changed(&self.projected_tab_sources, state.tabs.as_slice()) {
            let projected_tabs = project_browser_tabs(state.tabs.as_slice());
            self.tabs
                .borrow_mut()
                .defer_replace_or_patch(projected_tabs);
            self.projected_tab_sources = project_browser_tab_sources(state.tabs.as_slice());
        }

        let frame_source = frame.as_ref().map(|frame| {
            (
                thread_id,
                self.active_tab_id.clone(),
                frame.metadata().frame_epoch,
            )
        });
        if frame_source != self.frame_source {
            if let Some(frame) = frame {
                self.publish_frame(&frame);
            } else {
                clear_browser_frame();
            }
            self.frame_source = frame_source;
        }
        if scalar_changed {
            self.notify_state_changed();
        }
    }

    fn publish_frame(&mut self, frame: &BrowserFrame) {
        self.presentation_epoch = self.presentation_epoch.wrapping_add(1).max(1);
        publish_browser_frame(frame, self.presentation_epoch);
    }

    fn clear_projection(&mut self) {
        self.projected_tab_sources.clear();
        self.tabs.borrow_mut().defer_replace_or_patch(Vec::new());
        self.active_tab_id.clear();
        self.active_tab_index = -1;
        self.url.clear();
        self.title.clear();
        self.loading = false;
        self.can_go_back = false;
        self.can_go_forward = false;
        self.status_message.clear();
        if self.frame_source.take().is_some() {
            clear_browser_frame();
        }
        self.notify_state_changed();
    }

    fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = message.into();
        self.notify_state_changed();
    }

    fn sync_context_menu(&mut self) {
        let Some(thread_id) = self.visible_thread_id() else {
            return;
        };
        let Some(target) = self.runtime.take_context_menu_target(thread_id.as_str()) else {
            return;
        };
        let x = target.x;
        let y = target.y;
        self.context_target_json = serde_json::to_string(&target).unwrap_or_default();
        self.context_target = Some(target);
        self.notify_context_changed();
        self.notify_context_menu_requested(x, y);
    }

    fn run_context_action(&mut self, action: &str) -> bool {
        let target = self.context_target.clone();
        let result = match action {
            "back" => self.apply_visible_action(BrowserAction::Back),
            "forward" => self.apply_visible_action(BrowserAction::Forward),
            "reload" => self.apply_visible_action(BrowserAction::Reload),
            "copy-page" => self.copy_context_text(
                target
                    .as_ref()
                    .and_then(|target| target.page_url.clone())
                    .or_else(|| (!self.url.is_empty()).then(|| self.url.clone())),
            ),
            "copy-link" => self.copy_context_text(target.and_then(|target| target.link_url)),
            "copy-media" => self.copy_context_text(target.and_then(|target| target.source_url)),
            "copy-selection" => {
                self.copy_context_text(target.and_then(|target| target.selection_text))
            }
            "open-link" => target
                .and_then(|target| target.link_url)
                .is_some_and(|url| self.open_context_url(url)),
            "open-media" => target
                .and_then(|target| target.source_url)
                .is_some_and(|url| self.open_context_url(url)),
            "cut" => self.apply_visible_action(BrowserAction::Press {
                keys: platform_shortcut("X"),
            }),
            "copy" => self.apply_visible_action(BrowserAction::Press {
                keys: platform_shortcut("C"),
            }),
            "paste" => self.apply_visible_action(BrowserAction::Press {
                keys: platform_shortcut("V"),
            }),
            "select-all" => self.apply_visible_action(BrowserAction::Press {
                keys: platform_shortcut("A"),
            }),
            "inspect" => {
                let Some(thread_id) = self.visible_thread_id() else {
                    return false;
                };
                let point = target.map(|target| BrowserPhysicalPoint {
                    x: target.x,
                    y: target.y,
                });
                self.runtime
                    .show_devtools(thread_id.as_str(), point)
                    .is_ok()
            }
            _ => false,
        };
        if result {
            self.context_target = None;
            self.context_target_json.clear();
            self.notify_context_changed();
        }
        result
    }

    fn copy_context_text(&mut self, text: Option<String>) -> bool {
        let Some(text) = text.filter(|text| !text.trim().is_empty()) else {
            return false;
        };
        self.context_clipboard_text = text;
        self.notify_context_changed();
        true
    }

    fn open_context_url(&mut self, url: String) -> bool {
        let Some(thread_id) = self.visible_thread_id() else {
            return false;
        };
        let tab_id = self
            .runtime
            .create_tab(thread_id.as_str(), Some(url.clone()), true);
        if let Err(error) = self
            .runtime
            .navigate_backend_tab(thread_id.as_str(), &tab_id, url)
        {
            let _ = self.runtime.close_tab(thread_id.as_str(), &tab_id);
            self.set_status(error.to_string());
            return false;
        }
        self.sync_projection();
        true
    }
}

fn browser_mouse_input(
    x: i32,
    y: i32,
    shift: bool,
    control: bool,
    alt: bool,
    meta: bool,
) -> BrowserMouseInput {
    BrowserMouseInput {
        point: BrowserPhysicalPoint {
            x: x.max(0),
            y: y.max(0),
        },
        modifiers: BrowserInputModifiers {
            shift,
            control,
            alt,
            meta,
        },
    }
}

fn normalize_browser_address(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("://") || lower.starts_with("about:") || lower.starts_with("data:") {
        return Some(trimmed.to_owned());
    }
    let scheme = if lower.starts_with("localhost") || lower.starts_with("127.") {
        "http://"
    } else {
        "https://"
    };
    Some(format!("{scheme}{trimmed}"))
}

fn sensitive_action_label(action: hunk_browser::SensitiveBrowserAction) -> String {
    match action {
        hunk_browser::SensitiveBrowserAction::CredentialEntry => "CREDENTIAL ENTRY",
        hunk_browser::SensitiveBrowserAction::PaymentOrPurchase => "PAYMENT OR PURCHASE",
        hunk_browser::SensitiveBrowserAction::FileTransfer => "FILE TRANSFER",
        hunk_browser::SensitiveBrowserAction::ExternalProtocol => "EXTERNAL APPLICATION",
        hunk_browser::SensitiveBrowserAction::HighRiskFormSubmit => "FORM SUBMISSION",
    }
    .to_owned()
}

fn platform_shortcut(key: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("Meta+{key}")
    } else {
        format!("Control+{key}")
    }
}

#[cfg(feature = "cef-browser")]
fn browser_runtime_config() -> Result<BrowserRuntimeConfig, String> {
    let app_data_dir = hunk_domain::state::app_data_dir().map_err(|error| error.to_string())?;
    let storage_paths = BrowserStoragePaths::from_app_data_dir_with_profile_id(
        app_data_dir,
        default_browser_profile_id(),
    );
    storage_paths
        .ensure_directories()
        .map_err(|error| error.to_string())?;
    Ok(BrowserRuntimeConfig::new(
        default_browser_cef_runtime_dir(),
        default_browser_helper_executable_path(),
        storage_paths,
    ))
}

#[cfg(feature = "cef-browser")]
fn default_browser_profile_id() -> String {
    if let Ok(profile_id) = std::env::var("HUNK_BROWSER_PROFILE_ID")
        && !profile_id.trim().is_empty()
    {
        return profile_id;
    }
    let Ok(exe_path) = std::env::current_exe() else {
        return "default".to_owned();
    };
    if !exe_path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|part| matches!(part, "target" | "debug" | "release"))
    }) {
        return "default".to_owned();
    }
    format!(
        "dev-{:016x}",
        stable_browser_profile_hash(exe_path.to_string_lossy().as_bytes())
    )
}

#[cfg(feature = "cef-browser")]
fn stable_browser_profile_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

#[cfg(feature = "cef-browser")]
fn default_browser_cef_runtime_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(contents_dir) = macos_app_contents_dir(current_exe.as_path())
    {
        return contents_dir.join("Frameworks");
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(exe_dir) = current_exe.parent()
    {
        #[cfg(target_os = "linux")]
        if exe_dir.join("lib/libcef.so").is_file() {
            return exe_dir.join("lib");
        }
        #[cfg(target_os = "windows")]
        if exe_dir.join("libcef.dll").is_file() {
            return exe_dir.to_path_buf();
        }
    }

    let platform = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "macos"
    };
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("assets/browser-runtime/cef")
        .join(platform)
        .join("runtime")
}

#[cfg(feature = "cef-browser")]
fn default_browser_helper_executable_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(contents_dir) = macos_app_contents_dir(current_exe.as_path())
    {
        return contents_dir
            .join("Frameworks")
            .join(format!(
                "{}.app",
                hunk_browser_helper::MACOS_HELPER_BUNDLE_NAME
            ))
            .join("Contents/MacOS")
            .join(hunk_browser_helper::MACOS_HELPER_BUNDLE_NAME);
    }
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(exe_dir) = current_exe.parent()
    {
        return exe_dir.join(hunk_browser_helper::helper_executable_name());
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("target/debug")
        .join(hunk_browser_helper::helper_executable_name())
}

#[cfg(all(feature = "cef-browser", target_os = "macos"))]
fn macos_app_contents_dir(current_exe: &Path) -> Option<PathBuf> {
    let macos_dir = current_exe.parent()?;
    if macos_dir.file_name()? != "MacOS" {
        return None;
    }
    let contents_dir = macos_dir.parent()?;
    (contents_dir.file_name()? == "Contents").then(|| contents_dir.to_path_buf())
}
