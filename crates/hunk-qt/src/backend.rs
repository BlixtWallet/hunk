use qtbridge::{QObjectHolder, invoke_method, qobject, qtbridge_type_lib::QString};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Workspace {
    Diff,
    Git,
    Ai,
}

impl Workspace {
    pub const ALL: [Self; 3] = [Self::Diff, Self::Git, Self::Ai];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Diff => "diff",
            Self::Git => "git",
            Self::Ai => "ai",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "diff" => Some(Self::Diff),
            "git" => Some(Self::Git),
            "ai" => Some(Self::Ai),
            _ => None,
        }
    }
}

pub struct Backend {
    active_workspace: String,
    ready: bool,
    status_message: String,
    bootstrap_started: bool,
}

impl Default for Backend {
    fn default() -> Self {
        Self {
            active_workspace: Workspace::Diff.as_str().to_owned(),
            ready: false,
            status_message: "Connecting Rust application services…".to_owned(),
            bootstrap_started: false,
        }
    }
}

#[qobject]
impl Backend {
    qproperty!(
        "activeWorkspace",
        Member = active_workspace,
        Notify = active_workspace_changed
    );
    qproperty!("ready", Member = ready, Notify = ready_changed);
    qproperty!(
        "statusMessage",
        Member = status_message,
        Notify = status_message_changed
    );

    #[qsignal]
    fn active_workspace_changed(&mut self);

    #[qsignal]
    fn ready_changed(&mut self);

    #[qsignal]
    fn status_message_changed(&mut self);

    #[qslot]
    fn select_workspace(&mut self, workspace: String) {
        let Some(workspace) = Workspace::parse(&workspace) else {
            self.set_status_message(format!("Unknown workspace: {workspace}"));
            return;
        };
        if self.active_workspace == workspace.as_str() {
            return;
        }

        self.active_workspace = workspace.as_str().to_owned();
        self.active_workspace_changed();
    }

    #[qslot]
    fn bootstrap(&mut self) {
        if self.bootstrap_started {
            return;
        }
        self.bootstrap_started = true;

        let invoker = self.get_qml_method_invoker();
        let spawn_result = std::thread::Builder::new()
            .name("hunk-qt-bootstrap".to_owned())
            .spawn(move || {
                invoke_method!(
                    invoker,
                    "complete_bootstrap",
                    true,
                    QString::from("Qt shell connected to Rust")
                );
            });

        if let Err(error) = spawn_result {
            self.complete_bootstrap(
                false,
                format!("Failed to start application services: {error}"),
            );
        }
    }

    #[qslot]
    fn complete_bootstrap(&mut self, ready: bool, status_message: String) {
        if self.ready != ready {
            self.ready = ready;
            self.ready_changed();
        }
        self.set_status_message(status_message);
    }

    fn set_status_message(&mut self, status_message: String) {
        if self.status_message == status_message {
            return;
        }
        self.status_message = status_message;
        self.status_message_changed();
    }
}
