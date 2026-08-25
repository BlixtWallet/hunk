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
    ListModel { id: diffFilesModel }
    ListModel { id: diffRowsModel }

    QtObject {
        id: fakeBackend

        property string activeWorkspace: "diff"
        property bool ready: true
        property string statusMessage: "Test backend ready"
        property string lastRequestedWorkspace: ""

        property var diffFiles: diffFilesModel
        property var diffRows: diffRowsModel
        property string diffSelectedPath: "crates/hunk-qt/src/backend.rs"
        property string diffStatusTag: "M"
        property int diffAdditions: 132
        property int diffRemovals: 8
        property bool diffReady: true
        property bool diffLoading: false
        property string diffError: ""
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
        property bool forgeAvailable: true
        property string forgeProviderLabel: "GitHub"
        property string forgeReviewKindLabel: "Pull Request"
        property string forgeHost: "github.com"
        property string forgeRepositoryPath: "smolcars/hunk"
        property bool forgeAuthenticated: true
        property string forgeAccountLabel: "Nitesh"
        property string forgeAuthMode: "device"
        property bool forgeReady: true
        property bool forgeLoading: false
        property bool forgeBusy: false
        property string forgeError: ""
        property string forgeStatusMessage: "GitHub connected"
        property string forgeActionLabel: ""
        property string forgeDefaultTargetBranch: "master"
        property bool forgeReviewExists: false
        property int forgeReviewNumber: 0
        property string forgeReviewTitle: ""
        property string forgeReviewUrl: ""
        property string forgeReviewState: ""
        property bool forgeReviewDraft: false
        property bool forgeDeviceFlowActive: false
        property string forgeDeviceUserCode: ""
        property string forgeDeviceVerificationUrl: ""
        property string lastCommand: ""
        property string lastArgument: ""
        property string lastTargetBranch: ""
        property string lastReviewTitle: ""
        property string lastReviewBody: ""
        property bool lastReviewDraft: false

        function record(command, argument) {
            lastCommand = command
            lastArgument = argument || ""
        }

        function select_workspace(workspace) {
            lastRequestedWorkspace = workspace
            activeWorkspace = workspace
        }

        function refresh_git_workspace() { record("refresh") }
        function select_diff_file(path) {
            record("select_diff_file", path)
            diffSelectedPath = path
        }
        function refresh_diff() { record("refresh_diff") }
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
        function refresh_forge_review() { record("refresh_forge") }
        function save_forge_personal_access_token(token) { record("save_forge_token", token) }
        function start_github_device_flow() { record("start_github_device_flow") }
        function cancel_github_device_flow() { record("cancel_github_device_flow") }
        function create_forge_review(targetBranch, title, body, draft) {
            record("create_forge_review")
            lastTargetBranch = targetBranch
            lastReviewTitle = title
            lastReviewBody = body
            lastReviewDraft = draft
        }
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
        diffFilesModel.clear()
        diffRowsModel.clear()

        appendFile("crates/hunk-git/src/workspace.rs", false, 84, 4)
        appendFile("crates/hunk-qt/src/backend.rs", false, 132, 8)
        appendFile("crates/hunk-qt/src/qml/Hunk/GitWorkspace.qml", true, 635, 0)
        diffFilesModel.append({
            path: "crates/hunk-qt/src/backend.rs",
            file_name: "backend.rs",
            directory: "crates/hunk-qt/src",
            status_tag: "M",
            status_label: "Modified",
            section: "",
            staged: false,
            additions: 132,
            removals: 8
        })
        diffFilesModel.append({
            path: "crates/hunk-qt/src/qml/Hunk/DiffWorkspace.qml",
            file_name: "DiffWorkspace.qml",
            directory: "crates/hunk-qt/src/qml/Hunk",
            status_tag: "A",
            status_label: "Added",
            section: "",
            staged: false,
            additions: 318,
            removals: 0
        })
        diffRowsModel.append({
            stable_id: "backend.rs:0:hunk",
            row_kind: "hunk",
            left_line: 0,
            left_text: "",
            left_kind: "none",
            right_line: 0,
            right_text: "",
            right_kind: "none",
            text: "@@ -22,7 +22,8 @@ impl Backend"
        })
        diffRowsModel.append({
            stable_id: "backend.rs:1:code",
            row_kind: "code",
            left_line: 22,
            left_text: "    qproperty!(\"gitFiles\", Read = git_files, Constant);",
            left_kind: "context",
            right_line: 22,
            right_text: "    qproperty!(\"gitFiles\", Read = git_files, Constant);",
            right_kind: "context",
            text: ""
        })
        diffRowsModel.append({
            stable_id: "backend.rs:2:code",
            row_kind: "code",
            left_line: 23,
            left_text: "    qproperty!(\"gitBranches\", Read = git_branches, Constant);",
            left_kind: "removed",
            right_line: 23,
            right_text: "    qproperty!(\"diffRows\", Read = diff_rows, Constant);",
            right_kind: "added",
            text: ""
        })
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

    function openDiffWorkspace() {
        shell.activateWorkspace("diff")
        tryVerify(() => shell.workspaceItem !== null && shell.workspaceItem.objectName === "diffWorkspace")
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
        fakeBackend.diffSelectedPath = "crates/hunk-qt/src/backend.rs"
        fakeBackend.diffStatusTag = "M"
        fakeBackend.diffAdditions = 132
        fakeBackend.diffRemovals = 8
        fakeBackend.diffReady = true
        fakeBackend.diffLoading = false
        fakeBackend.diffError = ""
        fakeBackend.gitChangedFileCount = 3
        fakeBackend.gitStagedFileCount = 1
        fakeBackend.gitUnstagedFileCount = 2
        fakeBackend.gitReady = true
        fakeBackend.gitLoading = false
        fakeBackend.gitBusy = false
        fakeBackend.gitError = ""
        fakeBackend.gitStatusMessage = "Repository refreshed"
        fakeBackend.gitActionLabel = ""
        fakeBackend.forgeAvailable = true
        fakeBackend.forgeProviderLabel = "GitHub"
        fakeBackend.forgeReviewKindLabel = "Pull Request"
        fakeBackend.forgeHost = "github.com"
        fakeBackend.forgeRepositoryPath = "smolcars/hunk"
        fakeBackend.forgeAuthenticated = true
        fakeBackend.forgeAccountLabel = "Nitesh"
        fakeBackend.forgeAuthMode = "device"
        fakeBackend.forgeReady = true
        fakeBackend.forgeLoading = false
        fakeBackend.forgeBusy = false
        fakeBackend.forgeError = ""
        fakeBackend.forgeStatusMessage = "GitHub connected"
        fakeBackend.forgeActionLabel = ""
        fakeBackend.forgeDefaultTargetBranch = "master"
        fakeBackend.forgeReviewExists = false
        fakeBackend.forgeReviewNumber = 0
        fakeBackend.forgeReviewTitle = ""
        fakeBackend.forgeReviewUrl = ""
        fakeBackend.forgeReviewState = ""
        fakeBackend.forgeReviewDraft = false
        fakeBackend.forgeDeviceFlowActive = false
        fakeBackend.forgeDeviceUserCode = ""
        fakeBackend.forgeDeviceVerificationUrl = ""
        fakeBackend.lastTargetBranch = ""
        fakeBackend.lastReviewTitle = ""
        fakeBackend.lastReviewBody = ""
        fakeBackend.lastReviewDraft = false
        snapshotReady = false
        snapshotSaved = false
        populateModels()
    }

    function test_retainedWorkspaceContract() {
        compare(shell.workspaceCount, 3)
        compare(shell.workspaceIds, ["diff", "git", "ai"])
    }

    function test_diffWorkspaceUsesVirtualizedRustModels() {
        openDiffWorkspace()
        shell.workspaceItem.diffListView.forceLayout()
        compare(shell.workspaceItem.diffListView.count, 3)
        verify(shell.workspaceItem.diffListView.reuseItems)
        compare(shell.sidebarItem.fileListView.count, 2)
        verify(shell.sidebarItem.fileListView.reuseItems)
    }

    function test_diffFileSelectionUsesBackendCommand() {
        openDiffWorkspace()
        shell.sidebarItem.fileListView.forceLayout()
        const secondFile = shell.sidebarItem.fileListView.itemAtIndex(1)
        verify(secondFile !== null)
        shell.sidebarItem.activateFile("crates/hunk-qt/src/qml/Hunk/DiffWorkspace.qml")
        compare(fakeBackend.lastCommand, "select_diff_file")
        compare(fakeBackend.lastArgument, "crates/hunk-qt/src/qml/Hunk/DiffWorkspace.qml")
    }

    function test_diffRowsRemainVirtualizedForLargePatches() {
        diffRowsModel.clear()
        for (let index = 0; index < 5000; ++index) {
            diffRowsModel.append({
                stable_id: "generated:" + index + ":code",
                row_kind: "code",
                left_line: index + 1,
                left_text: "let before_" + index + " = value;",
                left_kind: "removed",
                right_line: index + 1,
                right_text: "let after_" + index + " = value;",
                right_kind: "added",
                text: ""
            })
        }

        openDiffWorkspace()
        shell.workspaceItem.diffListView.forceLayout()
        compare(shell.workspaceItem.diffListView.count, 5000)
        verify(shell.workspaceItem.diffListView.itemAtIndex(0) !== null)
        verify(shell.workspaceItem.diffListView.itemAtIndex(1000) === null)
    }

    function test_diffWorkspaceRendersAtDesktopSize() {
        openDiffWorkspace()
        captureSnapshot("target/hunk-qt-diff.png")
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

    function test_githubDeviceAuthenticationUsesRustCommand() {
        fakeBackend.forgeAuthenticated = false
        openGitWorkspace()

        shell.workspaceItem.requestForgeAuthentication()

        compare(fakeBackend.lastCommand, "start_github_device_flow")
        verify(!shell.workspaceItem.forgeTokenDialogVisible)
    }

    function test_personalAccessTokenIsClearedAfterSubmission() {
        fakeBackend.forgeAuthenticated = false
        fakeBackend.forgeProviderLabel = "GitLab"
        fakeBackend.forgeReviewKindLabel = "Merge Request"
        fakeBackend.forgeAuthMode = "token"
        openGitWorkspace()

        shell.workspaceItem.requestForgeAuthentication()
        verify(shell.workspaceItem.forgeTokenDialogVisible)
        captureSnapshot("target/hunk-qt-forge-token.png")
        shell.workspaceItem.forgeTokenDialog.tokenInput.text = "secret-token"
        shell.workspaceItem.forgeTokenDialog.submitted("secret-token")

        compare(fakeBackend.lastCommand, "save_forge_token")
        compare(fakeBackend.lastArgument, "secret-token")
        verify(!shell.workspaceItem.forgeTokenDialogVisible)
        compare(shell.workspaceItem.forgeTokenDialog.tokenInput.text, "")
    }

    function test_reviewDialogSubmitsFindOrCreateFields() {
        openGitWorkspace()
        shell.workspaceItem.openForgeReviewDialog()
        verify(shell.workspaceItem.forgeReviewDialogVisible)
        captureSnapshot("target/hunk-qt-forge-review.png")
        compare(shell.workspaceItem.forgeReviewDialog.targetInput.text, "master")
        compare(
            shell.workspaceItem.forgeReviewDialog.titleInput.text,
            "Connect QtBridge to the Rust Git core"
        )

        shell.workspaceItem.forgeReviewDialog.titleInput.text = "Qt forge controls"
        shell.workspaceItem.forgeReviewDialog.bodyInput.text = "Move review actions to Qt."
        shell.workspaceItem.forgeReviewDialog.draft = true
        shell.workspaceItem.forgeReviewDialog.submitted(
            "master",
            "Qt forge controls",
            "Move review actions to Qt.",
            true
        )

        compare(fakeBackend.lastCommand, "create_forge_review")
        compare(fakeBackend.lastTargetBranch, "master")
        compare(fakeBackend.lastReviewTitle, "Qt forge controls")
        compare(fakeBackend.lastReviewBody, "Move review actions to Qt.")
        verify(fakeBackend.lastReviewDraft)
        verify(!shell.workspaceItem.forgeReviewDialogVisible)
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
