import QtQuick
import QtTest
import Hunk 1.0

TestCase {
    name: "HunkShell"

    property bool snapshotReady: false
    property bool snapshotSaved: false

    ListModel { id: gitFilesModel }
    ListModel { id: gitBranchesModel }
    ListModel { id: gitCommitsModel }

    QtObject {
        id: fakeBackend

        property string activeWorkspace: "diff"
        property bool ready: true
        property string statusMessage: "Test backend ready"
        property string lastRequestedWorkspace: ""

        property var gitFiles: gitFilesModel
        property var gitBranches: gitBranchesModel
        property var gitCommits: gitCommitsModel
        property string gitRoot: "/Volumes/hulk/dev/projects/hunk"
        property string gitRepositoryName: "hunk"
        property string gitBranchName: "migration/05-qt-git"
        property bool gitBranchHasUpstream: true
        property int gitBranchAheadCount: 2
        property int gitBranchBehindCount: 0
        property int gitChangedFileCount: 3
        property int gitStagedFileCount: 1
        property int gitUnstagedFileCount: 2
        property string gitLastCommitSubject: "Connect QtBridge to the Rust Git core"
        property bool gitReady: true
        property bool gitLoading: false
        property bool gitBusy: false
        property string gitError: ""
        property string gitStatusMessage: "Repository refreshed"
        property string gitActionLabel: ""
        property string lastCommand: ""
        property string lastArgument: ""

        function record(command, argument) {
            lastCommand = command
            lastArgument = argument || ""
        }

        function select_workspace(workspace) {
            lastRequestedWorkspace = workspace
            activeWorkspace = workspace
        }

        function refresh_git_workspace() { record("refresh") }
        function select_git_root(root) { record("select_root", root) }
        function stage_path(path) { record("stage", path) }
        function unstage_path(path) { record("unstage", path) }
        function stage_all() { record("stage_all") }
        function unstage_all() { record("unstage_all") }
        function discard_path(path) { record("discard", path) }
        function commit_staged(message) { record("commit", message) }
        function activate_branch(name) { record("activate_branch", name) }
        function fetch_remote_branches() { record("fetch") }
        function publish_branch() { record("publish") }
        function push_branch() { record("push") }
        function sync_branch() { record("sync") }
        function pull_branch_with_rebase() { record("pull_rebase") }
    }

    Shell {
        id: shell
        width: 1280
        height: 760
        backend: fakeBackend
    }

    function appendFile(path, staged, additions, removals) {
        const slash = path.lastIndexOf("/")
        gitFilesModel.append({
            path: path,
            file_name: slash >= 0 ? path.slice(slash + 1) : path,
            directory: slash >= 0 ? path.slice(0, slash) : "",
            status_tag: staged ? "A" : "M",
            status_label: staged ? "Added" : "Modified",
            section: staged ? "STAGED" : "CHANGES",
            staged: staged,
            additions: additions,
            removals: removals
        })
    }

    function populateModels() {
        gitFilesModel.clear()
        gitBranchesModel.clear()
        gitCommitsModel.clear()

        appendFile("crates/hunk-git/src/workspace.rs", false, 84, 4)
        appendFile("crates/hunk-qt/src/backend.rs", false, 132, 8)
        appendFile("crates/hunk-qt/src/qml/Hunk/GitWorkspace.qml", true, 635, 0)
        gitBranchesModel.append({
            name: "migration/05-qt-git",
            current: true,
            remote: false,
            workspace_label: ""
        })
        gitBranchesModel.append({
            name: "migration/04-qt-foundation",
            current: false,
            remote: false,
            workspace_label: ""
        })
        gitBranchesModel.append({
            name: "origin/master",
            current: false,
            remote: true,
            workspace_label: ""
        })
        gitCommitsModel.append({
            commit_id: "59b2f24303feef4e8d4b3dc17fdbe0f93e04feaa",
            short_id: "59b2f243",
            subject: "Move Qt Linux checks to cached ephemeral runners",
            committed_unix_time: 1787677200
        })
        gitCommitsModel.append({
            commit_id: "7037955a4b1b7db3a56aa409093beaabdf40fc43",
            short_id: "7037955a",
            subject: "Pin unreleased aqt Windows layout fix for Qt 6.11.2",
            committed_unix_time: 1787673600
        })
    }

    function openGitWorkspace() {
        shell.activateWorkspace("git")
        tryVerify(() => shell.workspaceItem !== null && shell.workspaceItem.objectName === "gitWorkspace")
    }

    function captureSnapshot(path) {
        snapshotReady = false
        snapshotSaved = false
        shell.grabToImage(function(result) {
            snapshotSaved = result.saveToFile(path)
            snapshotReady = true
        })
        tryVerify(() => snapshotReady)
        verify(snapshotSaved)
    }

    function init() {
        fakeBackend.activeWorkspace = "diff"
        wait(0)
        fakeBackend.lastRequestedWorkspace = ""
        fakeBackend.lastCommand = ""
        fakeBackend.lastArgument = ""
        fakeBackend.gitChangedFileCount = 3
        fakeBackend.gitStagedFileCount = 1
        fakeBackend.gitUnstagedFileCount = 2
        fakeBackend.gitReady = true
        fakeBackend.gitLoading = false
        fakeBackend.gitBusy = false
        fakeBackend.gitError = ""
        fakeBackend.gitStatusMessage = "Repository refreshed"
        fakeBackend.gitActionLabel = ""
        snapshotReady = false
        snapshotSaved = false
        populateModels()
    }

    function test_retainedWorkspaceContract() {
        compare(shell.workspaceCount, 3)
        compare(shell.workspaceIds, ["diff", "git", "ai"])
    }

    function test_workspaceActivationUsesBackendCommand() {
        openGitWorkspace()
        compare(fakeBackend.lastRequestedWorkspace, "git")
        compare(shell.activeWorkspace, "git")
    }

    function test_discardRequiresConfirmationBeforeRustCommand() {
        openGitWorkspace()
        shell.workspaceItem.requestDiscard("crates/hunk-git/src/workspace.rs")
        verify(shell.workspaceItem.discardConfirmationVisible)
        compare(fakeBackend.lastCommand, "")

        shell.workspaceItem.confirmDiscard()
        verify(!shell.workspaceItem.discardConfirmationVisible)
        compare(fakeBackend.lastCommand, "discard")
        compare(fakeBackend.lastArgument, "crates/hunk-git/src/workspace.rs")
    }

    function test_commitComposerUsesStagedCommitCommand() {
        openGitWorkspace()
        shell.workspaceItem.commitMessageInput.text = "Migrate the Git workspace"
        shell.workspaceItem.submitCommit()
        compare(fakeBackend.lastCommand, "commit")
        compare(fakeBackend.lastArgument, "Migrate the Git workspace")
        compare(shell.workspaceItem.commitMessageInput.text, "")
    }

    function test_fileListRemainsVirtualizedForLargeRepositories() {
        gitFilesModel.clear()
        for (let index = 0; index < 1500; ++index)
            appendFile("generated/path/file-" + index + ".rs", false, index % 7, index % 3)
        fakeBackend.gitChangedFileCount = 1500
        fakeBackend.gitStagedFileCount = 0
        fakeBackend.gitUnstagedFileCount = 1500

        openGitWorkspace()
        shell.workspaceItem.fileListView.forceLayout()
        compare(shell.workspaceItem.fileListView.count, 1500)
        verify(shell.workspaceItem.fileListView.reuseItems)
        verify(shell.workspaceItem.fileListView.itemAtIndex(0) !== null)
        verify(shell.workspaceItem.fileListView.itemAtIndex(1000) === null)
    }

    function test_gitWorkspaceStatesCoverLoadingEmptyAndError() {
        gitFilesModel.clear()
        fakeBackend.gitChangedFileCount = 0
        fakeBackend.gitStagedFileCount = 0
        fakeBackend.gitUnstagedFileCount = 0
        openGitWorkspace()
        verify(shell.workspaceItem.emptyStateVisible)
        captureSnapshot("target/hunk-qt-git-empty.png")

        fakeBackend.gitReady = false
        fakeBackend.gitLoading = true
        verify(shell.workspaceItem.loadingStateVisible)
        captureSnapshot("target/hunk-qt-git-loading.png")

        fakeBackend.gitLoading = false
        fakeBackend.gitError = "Unable to open repository"
        verify(shell.workspaceItem.errorStateVisible)
        captureSnapshot("target/hunk-qt-git-error.png")
    }

    function test_gitWorkspaceRendersAtDesktopSize() {
        openGitWorkspace()
        captureSnapshot("target/hunk-qt-git.png")
    }
}
