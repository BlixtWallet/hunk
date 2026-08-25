use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use hunk_domain::state::AppStateStore;
use hunk_git::workspace::{GitWorkspaceCommand, execute_git_workspace_command, load_git_workspace};
use qtbridge::{QObjectHolder, invoke_method, qobject, qtbridge_type_lib::QString};

use crate::git_models::{
    GitBranchListModel, GitCommitListModel, GitFileListModel, GitSnapshotPayload,
};
use crate::local_path_from_qml_folder_url;

type GitRefreshResult = Result<GitSnapshotPayload, String>;

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
    git_files: Rc<RefCell<GitFileListModel>>,
    git_branches: Rc<RefCell<GitBranchListModel>>,
    git_commits: Rc<RefCell<GitCommitListModel>>,
    git_root: String,
    git_repository_name: String,
    git_branch_name: String,
    git_branch_has_upstream: bool,
    git_branch_ahead_count: i32,
    git_branch_behind_count: i32,
    git_changed_file_count: i32,
    git_staged_file_count: i32,
    git_unstaged_file_count: i32,
    git_last_commit_subject: String,
    git_ready: bool,
    git_loading: bool,
    git_busy: bool,
    git_error: String,
    git_status_message: String,
    git_action_label: String,
    git_epoch: i32,
    git_refresh_results: Arc<Mutex<HashMap<i32, GitRefreshResult>>>,
    git_root_pending_persist: bool,
    git_staged_paths: Vec<String>,
    git_unstaged_paths: Vec<String>,
}

impl Default for Backend {
    fn default() -> Self {
        let git_root = initial_git_root();
        Self {
            active_workspace: Workspace::Diff.as_str().to_owned(),
            ready: false,
            status_message: "Connecting Rust application services…".to_owned(),
            bootstrap_started: false,
            git_files: GitFileListModel::default_with_attached_qobject(),
            git_branches: GitBranchListModel::default_with_attached_qobject(),
            git_commits: GitCommitListModel::default_with_attached_qobject(),
            git_root: git_root.display().to_string(),
            git_repository_name: "Repository".to_owned(),
            git_branch_name: String::new(),
            git_branch_has_upstream: false,
            git_branch_ahead_count: 0,
            git_branch_behind_count: 0,
            git_changed_file_count: 0,
            git_staged_file_count: 0,
            git_unstaged_file_count: 0,
            git_last_commit_subject: String::new(),
            git_ready: false,
            git_loading: false,
            git_busy: false,
            git_error: String::new(),
            git_status_message: String::new(),
            git_action_label: String::new(),
            git_epoch: 0,
            git_refresh_results: Arc::new(Mutex::new(HashMap::new())),
            git_root_pending_persist: false,
            git_staged_paths: Vec::new(),
            git_unstaged_paths: Vec::new(),
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
    qproperty!("gitFiles", Read = git_files, Constant);
    qproperty!("gitBranches", Read = git_branches, Constant);
    qproperty!("gitCommits", Read = git_commits, Constant);
    qproperty!("gitRoot", Member = git_root, Notify = git_state_changed);
    qproperty!(
        "gitRepositoryName",
        Member = git_repository_name,
        Notify = git_state_changed
    );
    qproperty!(
        "gitBranchName",
        Member = git_branch_name,
        Notify = git_state_changed
    );
    qproperty!(
        "gitBranchHasUpstream",
        Member = git_branch_has_upstream,
        Notify = git_state_changed
    );
    qproperty!(
        "gitBranchAheadCount",
        Member = git_branch_ahead_count,
        Notify = git_state_changed
    );
    qproperty!(
        "gitBranchBehindCount",
        Member = git_branch_behind_count,
        Notify = git_state_changed
    );
    qproperty!(
        "gitChangedFileCount",
        Member = git_changed_file_count,
        Notify = git_state_changed
    );
    qproperty!(
        "gitStagedFileCount",
        Member = git_staged_file_count,
        Notify = git_state_changed
    );
    qproperty!(
        "gitUnstagedFileCount",
        Member = git_unstaged_file_count,
        Notify = git_state_changed
    );
    qproperty!(
        "gitLastCommitSubject",
        Member = git_last_commit_subject,
        Notify = git_state_changed
    );
    qproperty!("gitReady", Member = git_ready, Notify = git_state_changed);
    qproperty!(
        "gitLoading",
        Member = git_loading,
        Notify = git_state_changed
    );
    qproperty!("gitBusy", Member = git_busy, Notify = git_state_changed);
    qproperty!("gitError", Member = git_error, Notify = git_state_changed);
    qproperty!(
        "gitStatusMessage",
        Member = git_status_message,
        Notify = git_state_changed
    );
    qproperty!(
        "gitActionLabel",
        Member = git_action_label,
        Notify = git_state_changed
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

    #[qsignal]
    fn git_state_changed(&mut self);

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
        if ready {
            self.refresh_git_workspace();
        }
    }

    #[qslot]
    fn refresh_git_workspace(&mut self) {
        if self.git_loading || self.git_busy {
            return;
        }

        self.git_epoch = self.git_epoch.wrapping_add(1).max(1);
        let epoch = self.git_epoch;
        self.git_loading = true;
        self.git_error.clear();
        self.git_state_changed();

        let root = PathBuf::from(self.git_root.clone());
        let invoker = self.get_qml_method_invoker();
        let refresh_results = Arc::clone(&self.git_refresh_results);
        let spawn_result = std::thread::Builder::new()
            .name("hunk-qt-git-refresh".to_owned())
            .spawn(move || {
                let result = load_git_workspace(root.as_path())
                    .map(GitSnapshotPayload::from)
                    .map_err(|error| format!("{error:#}"));
                if let Ok(mut pending) = refresh_results.lock() {
                    pending.insert(epoch, result);
                }
                invoke_method!(invoker, "apply_git_snapshot", epoch);
            });

        if let Err(error) = spawn_result {
            self.git_loading = false;
            self.git_error = format!("Failed to start Git refresh: {error}");
            self.git_state_changed();
        }
    }

    #[qslot]
    fn select_git_root(&mut self, root: String) {
        if self.git_busy {
            self.git_error =
                "Wait for the current Git operation before switching repositories".to_owned();
            self.git_state_changed();
            return;
        }
        let root = match local_path_from_qml_folder_url(root.as_str()) {
            Ok(root) => root,
            Err(error) => {
                self.git_error = error;
                self.git_state_changed();
                return;
            }
        };
        if !root.is_dir() {
            self.git_error = format!("Repository folder does not exist: {}", root.display());
            self.git_state_changed();
            return;
        }
        if root == Path::new(self.git_root.as_str()) {
            return;
        }

        self.git_epoch = self.git_epoch.wrapping_add(1).max(1);
        self.git_loading = false;
        self.git_root = root.display().to_string();
        self.git_repository_name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.git_root.clone());
        self.git_branch_name.clear();
        self.git_branch_has_upstream = false;
        self.git_branch_ahead_count = 0;
        self.git_branch_behind_count = 0;
        self.git_changed_file_count = 0;
        self.git_staged_file_count = 0;
        self.git_unstaged_file_count = 0;
        self.git_last_commit_subject.clear();
        self.git_ready = false;
        self.git_error.clear();
        self.git_status_message.clear();
        self.git_root_pending_persist = true;
        self.git_staged_paths.clear();
        self.git_unstaged_paths.clear();
        self.git_files.borrow_mut().replace(Vec::new());
        self.git_branches.borrow_mut().replace(Vec::new());
        self.git_commits.borrow_mut().replace(Vec::new());
        self.git_state_changed();
        self.refresh_git_workspace();
    }

    #[qslot]
    fn apply_git_snapshot(&mut self, epoch: i32) {
        let result = self
            .git_refresh_results
            .lock()
            .ok()
            .and_then(|mut pending| pending.remove(&epoch));
        if epoch != self.git_epoch {
            return;
        }
        self.git_loading = false;
        let payload = match result {
            Some(Ok(payload)) => payload,
            Some(Err(error)) => {
                self.git_error = error;
                self.git_state_changed();
                return;
            }
            None => {
                self.git_error = "Git refresh completed without a queued result".to_owned();
                self.git_state_changed();
                return;
            }
        };
        self.apply_git_payload(payload);
    }

    #[qslot]
    fn stage_path(&mut self, path: String) {
        self.run_git_command("Staging file", GitWorkspaceCommand::StagePaths(vec![path]));
    }

    #[qslot]
    fn unstage_path(&mut self, path: String) {
        self.run_git_command(
            "Unstaging file",
            GitWorkspaceCommand::UnstagePaths(vec![path]),
        );
    }

    #[qslot]
    fn stage_all(&mut self) {
        self.run_git_command(
            "Staging files",
            GitWorkspaceCommand::StagePaths(self.git_unstaged_paths.clone()),
        );
    }

    #[qslot]
    fn unstage_all(&mut self) {
        self.run_git_command(
            "Unstaging files",
            GitWorkspaceCommand::UnstagePaths(self.git_staged_paths.clone()),
        );
    }

    #[qslot]
    fn discard_path(&mut self, path: String) {
        self.run_git_command(
            "Discarding changes",
            GitWorkspaceCommand::RestorePaths(vec![path]),
        );
    }

    #[qslot]
    fn commit_staged(&mut self, message: String) {
        self.run_git_command(
            "Creating commit",
            GitWorkspaceCommand::CommitStaged { message },
        );
    }

    #[qslot]
    fn activate_branch(&mut self, name: String) {
        self.run_git_command(
            "Activating branch",
            GitWorkspaceCommand::ActivateBranch { name },
        );
    }

    #[qslot]
    fn fetch_remote_branches(&mut self) {
        self.run_git_command(
            "Fetching branches",
            GitWorkspaceCommand::FetchRemoteBranches,
        );
    }

    #[qslot]
    fn publish_branch(&mut self) {
        self.run_git_command(
            "Publishing branch",
            GitWorkspaceCommand::PublishBranch {
                name: self.git_branch_name.clone(),
            },
        );
    }

    #[qslot]
    fn push_branch(&mut self) {
        self.run_git_command(
            "Pushing branch",
            GitWorkspaceCommand::PushBranch {
                name: self.git_branch_name.clone(),
            },
        );
    }

    #[qslot]
    fn sync_branch(&mut self) {
        self.run_git_command(
            "Syncing branch",
            GitWorkspaceCommand::SyncBranch {
                name: self.git_branch_name.clone(),
            },
        );
    }

    #[qslot]
    fn pull_branch_with_rebase(&mut self) {
        self.run_git_command(
            "Rebasing branch",
            GitWorkspaceCommand::PullBranchWithRebase {
                name: self.git_branch_name.clone(),
            },
        );
    }

    #[qslot]
    fn complete_git_command(&mut self, success: bool, message: String) {
        self.git_busy = false;
        self.git_action_label.clear();
        if success {
            self.git_status_message = message;
            self.git_state_changed();
            self.refresh_git_workspace();
        } else {
            self.git_error = message;
            self.git_state_changed();
        }
    }

    fn set_status_message(&mut self, status_message: String) {
        if self.status_message == status_message {
            return;
        }
        self.status_message = status_message;
        self.status_message_changed();
    }

    fn git_files(&self) -> Rc<RefCell<GitFileListModel>> {
        self.git_files.clone()
    }

    fn git_branches(&self) -> Rc<RefCell<GitBranchListModel>> {
        self.git_branches.clone()
    }

    fn git_commits(&self) -> Rc<RefCell<GitCommitListModel>> {
        self.git_commits.clone()
    }

    fn apply_git_payload(&mut self, payload: GitSnapshotPayload) {
        self.git_staged_paths = payload
            .files
            .iter()
            .filter(|file| file.staged)
            .map(|file| file.path.clone())
            .collect();
        self.git_unstaged_paths = payload
            .files
            .iter()
            .filter(|file| !file.staged)
            .map(|file| file.path.clone())
            .collect();
        self.git_files.borrow_mut().replace(payload.files);
        self.git_branches.borrow_mut().replace(payload.branches);
        self.git_commits.borrow_mut().replace(payload.commits);
        self.git_root = payload.root;
        self.git_repository_name = payload.repository_name;
        self.git_branch_name = payload.branch_name;
        self.git_branch_has_upstream = payload.branch_has_upstream;
        self.git_branch_ahead_count = payload.branch_ahead_count;
        self.git_branch_behind_count = payload.branch_behind_count;
        self.git_changed_file_count = payload.changed_file_count;
        self.git_staged_file_count = payload.staged_file_count;
        self.git_unstaged_file_count = payload.unstaged_file_count;
        self.git_last_commit_subject = payload.last_commit_subject;
        self.git_ready = true;
        self.git_error.clear();
        if self.git_root_pending_persist {
            self.git_root_pending_persist = false;
            if let Err(error) = persist_active_project(PathBuf::from(self.git_root.as_str())) {
                self.git_status_message =
                    format!("Repository loaded; failed to save selection: {error:#}");
            }
        }
        self.git_state_changed();
    }

    fn run_git_command(&mut self, label: &str, command: GitWorkspaceCommand) {
        if self.git_loading || self.git_busy {
            return;
        }
        self.git_busy = true;
        self.git_error.clear();
        self.git_action_label = label.to_owned();
        self.git_state_changed();

        let root = PathBuf::from(self.git_root.clone());
        let invoker = self.get_qml_method_invoker();
        let spawn_result = std::thread::Builder::new()
            .name("hunk-qt-git-command".to_owned())
            .spawn(move || {
                let result = execute_git_workspace_command(root.as_path(), command);
                let (success, message) = match result {
                    Ok(outcome) => (true, outcome.message),
                    Err(error) => (false, format!("{error:#}")),
                };
                invoke_method!(
                    invoker,
                    "complete_git_command",
                    success,
                    QString::from(message)
                );
            });

        if let Err(error) = spawn_result {
            self.git_busy = false;
            self.git_action_label.clear();
            self.git_error = format!("Failed to start Git command: {error}");
            self.git_state_changed();
        }
    }
}

fn initial_git_root() -> PathBuf {
    AppStateStore::new()
        .and_then(|store| store.load_or_default())
        .ok()
        .and_then(|state| state.active_project_path().cloned())
        .filter(|path| path.is_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn persist_active_project(root: PathBuf) -> anyhow::Result<()> {
    let store = AppStateStore::new()?;
    let mut state = store.load_or_default()?;
    state.activate_workspace_project(root);
    store.save(&state)
}
