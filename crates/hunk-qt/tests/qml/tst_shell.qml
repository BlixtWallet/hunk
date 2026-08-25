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
    ListModel { id: diffCommentsModel }
    ListModel { id: aiThreadsModel }
    ListModel { id: aiTimelineModel }

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
        property string diffSearchQuery: ""
        property int diffSearchMatchCount: 0
        property int diffSearchMatchIndex: -1
        property int diffSearchTargetRow: -1
        property var diffSearchMatches: []
        property var diffComments: diffCommentsModel
        property bool diffCommentsReady: true
        property bool diffCommentsLoading: false
        property bool diffCommentsBusy: false
        property string diffCommentsError: ""
        property string diffCommentsStatusMessage: ""
        property bool diffCommentsShowNonOpen: false
        property int diffCommentsOpenCount: 0
        property int diffCommentsStaleCount: 0
        property int diffCommentsResolvedCount: 0
        property int diffCommentsVersion: 0
        property int diffCommentTargetRow: -1
        property int diffCommentTargetRevision: 0
        property var diffCommentRecords: []
        property bool failNextDiffComment: false
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
        property var aiThreads: aiThreadsModel
        property var aiTimeline: aiTimelineModel
        property bool aiReady: true
        property bool aiLoading: false
        property bool aiRequiresAuthentication: false
        property string aiConnectionState: "ready"
        property string aiWorkspaceRoot: "/Volumes/hulk/dev/projects/hunk"
        property string aiActiveThreadId: "thread-qt-migration"
        property string aiActiveThreadTitle: "Replace the GPUI AI workspace"
        property string aiActiveThreadCwd: "/Volumes/hulk/dev/projects/hunk"
        property int aiThreadCount: 2
        property int aiRunningThreadCount: 1
        property int aiTimelineTotalTurnCount: 2
        property int aiTimelineVisibleTurnCount: 2
        property int aiTimelineHiddenTurnCount: 0
        property int aiTimelineTotalRowCount: 4
        property int aiTimelineHiddenRowCount: 0
        property string aiError: ""
        property string aiStatusMessage: "Codex thread catalog refreshed"
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

        signal diffCommentsStateChanged

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
        function set_diff_search(query) {
            record("set_diff_search", query)
            diffSearchQuery = query
            const normalized = query.trim().toLowerCase()
            const matches = []
            if (normalized.length > 0) {
                for (let index = 0; index < diffRowsModel.count; ++index) {
                    const row = diffRowsModel.get(index)
                    const text = row.left_text + "\n" + row.right_text + "\n" + row.text
                    if (text.toLowerCase().includes(normalized))
                        matches.push(index)
                }
            }
            diffSearchMatches = matches
            diffSearchMatchCount = matches.length
            diffSearchMatchIndex = matches.length > 0 ? 0 : -1
            diffSearchTargetRow = matches.length > 0 ? matches[0] : -1
        }
        function move_diff_search_match(direction) {
            record("move_diff_search_match", String(direction))
            if (diffSearchMatches.length === 0)
                return
            diffSearchMatchIndex = (diffSearchMatchIndex + direction
                + diffSearchMatches.length) % diffSearchMatches.length
            diffSearchTargetRow = diffSearchMatches[diffSearchMatchIndex]
        }
        function diff_selection_text(anchor, head) {
            if (anchor < 0 || head < 0 || diffRowsModel.count === 0)
                return ""
            const start = Math.max(0, Math.min(anchor, head, diffRowsModel.count - 1))
            const end = Math.max(0, Math.min(Math.max(anchor, head), diffRowsModel.count - 1))
            const lines = []
            for (let index = start; index <= end; ++index) {
                const row = diffRowsModel.get(index)
                if (row.row_kind === "code") {
                    if (row.left_kind === "removed")
                        lines.push("-" + row.left_text)
                    if (row.right_kind === "added")
                        lines.push("+" + row.right_text)
                    if (row.left_kind === "context")
                        lines.push(" " + row.left_text)
                    if (row.left_kind === "none" && row.right_kind === "none"
                            && row.text.length > 0)
                        lines.push(row.text)
                } else if ((row.row_kind === "meta" || row.row_kind === "empty")
                        && row.text.length > 0) {
                    lines.push(row.text)
                }
            }
            return lines.join("\n")
        }
        function diff_hunk_target(start, direction) {
            const hunks = []
            for (let index = 0; index < diffRowsModel.count; ++index) {
                if (diffRowsModel.get(index).row_kind === "hunk")
                    hunks.push(index)
            }
            if (hunks.length === 0)
                return -1
            if (start < 0)
                return direction >= 0 ? hunks[0] : hunks[hunks.length - 1]
            if (direction >= 0) {
                for (const index of hunks) {
                    if (index > start)
                        return index
                }
                return hunks[0]
            }
            for (let index = hunks.length - 1; index >= 0; --index) {
                if (hunks[index] < start)
                    return hunks[index]
            }
            return hunks[hunks.length - 1]
        }
        function rebuildDiffComments() {
            diffCommentsModel.clear()
            let openCount = 0
            let staleCount = 0
            let resolvedCount = 0
            let visibleCount = 0
            for (const comment of diffCommentRecords) {
                if (comment.status === "open")
                    ++openCount
                else if (comment.status === "stale")
                    ++staleCount
                else
                    ++resolvedCount
                if ((diffCommentsShowNonOpen || comment.status === "open")
                        && visibleCount < 64) {
                    diffCommentsModel.append(comment)
                    ++visibleCount
                }
            }
            diffCommentsOpenCount = openCount
            diffCommentsStaleCount = staleCount
            diffCommentsResolvedCount = resolvedCount
            diffCommentsVersion += 1
            diffCommentsStateChanged()
        }
        function refresh_diff_comments() {
            record("refresh_diff_comments")
            diffCommentsReady = true
            rebuildDiffComments()
        }
        function create_diff_comment(row, text) {
            record("create_diff_comment", String(row) + "|" + text)
            diffCommentsBusy = true
            diffCommentsError = ""
            diffCommentsStateChanged()
            if (failNextDiffComment) {
                failNextDiffComment = false
                diffCommentsBusy = false
                diffCommentsError = "Failed to save comment"
                diffCommentsStatusMessage = ""
                diffCommentsStateChanged()
                return
            }
            const commentId = "comment-created-" + (diffCommentRecords.length + 1)
            diffCommentRecords = diffCommentRecords.concat([{
                comment_id: commentId,
                status: "open",
                file_path: diffSelectedPath,
                line_hint: diff_comment_line_hint(row),
                comment_text: text.trim(),
                clipboard_text: "[Hunk Comment]\nComment:\n" + text.trim(),
                row: row,
                can_jump: true
            }])
            diffCommentsStatusMessage = "Comment added."
            diffCommentsBusy = false
            rebuildDiffComments()
        }
        function set_diff_comment_status(commentId, status) {
            record("set_diff_comment_status", commentId + "|" + status)
            diffCommentRecords = diffCommentRecords.map(comment => {
                if (comment.comment_id === commentId)
                    comment.status = status
                return comment
            })
            diffCommentsStatusMessage = status === "open"
                ? "Comment reopened." : "Comment resolved."
            rebuildDiffComments()
        }
        function delete_diff_comment(commentId) {
            record("delete_diff_comment", commentId)
            diffCommentRecords = diffCommentRecords.filter(
                comment => comment.comment_id !== commentId
            )
            diffCommentsStatusMessage = "Comment deleted."
            rebuildDiffComments()
        }
        function set_diff_comments_show_non_open(show) {
            record("set_diff_comments_show_non_open", String(show))
            diffCommentsShowNonOpen = show
            rebuildDiffComments()
        }
        function jump_to_diff_comment(commentId) {
            record("jump_to_diff_comment", commentId)
            for (const comment of diffCommentRecords) {
                if (comment.comment_id === commentId && comment.row >= 0) {
                    diffCommentTargetRow = comment.row
                    diffCommentTargetRevision += 1
                    diffCommentsStatusMessage = "Jumped to comment location."
                    diffCommentsStateChanged()
                    return
                }
            }
        }
        function diff_comment_count_for_row(row) {
            let count = 0
            for (const comment of diffCommentRecords) {
                if (comment.status === "open" && comment.row === row)
                    ++count
            }
            return count
        }
        function diff_row_supports_comments(row) {
            if (row < 0 || row >= diffRowsModel.count)
                return false
            const kind = diffRowsModel.get(row).row_kind
            return kind === "code" || kind === "meta" || kind === "empty"
        }
        function diff_comment_line_hint(row) {
            if (row < 0 || row >= diffRowsModel.count)
                return ""
            const item = diffRowsModel.get(row)
            const oldLine = item.left_line > 0 ? item.left_line : "-"
            const newLine = item.right_line > 0 ? item.right_line : "-"
            return "old " + oldLine + " | new " + newLine
        }
        function diff_comment_bundle(commentId) {
            for (const comment of diffCommentRecords) {
                if (comment.comment_id === commentId)
                    return comment.clipboard_text
            }
            return ""
        }
        function diff_all_open_comment_bundles() {
            return diffCommentRecords
                .filter(comment => comment.status === "open")
                .map(comment => comment.clipboard_text)
                .join("\n\n---\n\n")
        }
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
        function refresh_ai_threads() { record("refresh_ai_threads") }
        function select_ai_thread(threadId) {
            record("select_ai_thread", threadId)
            for (let index = 0; index < aiThreadsModel.count; ++index) {
                const active = aiThreadsModel.get(index).thread_id === threadId
                aiThreadsModel.setProperty(index, "active", active)
                if (active) {
                    aiActiveThreadId = threadId
                    aiActiveThreadTitle = aiThreadsModel.get(index).title
                    aiActiveThreadCwd = aiThreadsModel.get(index).cwd
                }
            }
        }
        function create_ai_thread() { record("create_ai_thread") }
        function archive_ai_thread(threadId) {
            record("archive_ai_thread", threadId)
            for (let index = 0; index < aiThreadsModel.count; ++index) {
                if (aiThreadsModel.get(index).thread_id === threadId) {
                    aiThreadsModel.remove(index)
                    aiThreadCount -= 1
                    break
                }
            }
            if (aiActiveThreadId === threadId) {
                aiActiveThreadId = ""
                aiActiveThreadTitle = ""
                aiActiveThreadCwd = ""
                aiTimelineModel.clear()
                aiTimelineTotalRowCount = 0
            }
        }
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
        diffCommentsModel.clear()
        aiThreadsModel.clear()
        aiTimelineModel.clear()

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
            left_markup: "",
            left_kind: "none",
            right_line: 0,
            right_text: "",
            right_markup: "",
            right_kind: "none",
            text: "@@ -22,7 +22,8 @@ impl Backend"
        })
        diffRowsModel.append({
            stable_id: "backend.rs:1:code",
            row_kind: "code",
            left_line: 22,
            left_text: "    qproperty!(\"gitFiles\", Read = git_files, Constant);",
            left_markup: "<font color=\"@plain@\">&nbsp;&nbsp;&nbsp;&nbsp;</font><font color=\"@function@\"><b>qproperty!</b></font><font color=\"@plain@\">(&quot;gitFiles&quot;,&nbsp;Read&nbsp;=&nbsp;</font><font color=\"@variable@\">git_files</font><font color=\"@plain@\">,&nbsp;Constant);</font>",
            left_kind: "context",
            right_line: 22,
            right_text: "    qproperty!(\"gitFiles\", Read = git_files, Constant);",
            right_markup: "<font color=\"@plain@\">&nbsp;&nbsp;&nbsp;&nbsp;</font><font color=\"@function@\"><b>qproperty!</b></font><font color=\"@plain@\">(&quot;gitFiles&quot;,&nbsp;Read&nbsp;=&nbsp;</font><font color=\"@variable@\">git_files</font><font color=\"@plain@\">,&nbsp;Constant);</font>",
            right_kind: "context",
            text: ""
        })
        diffRowsModel.append({
            stable_id: "backend.rs:2:code",
            row_kind: "code",
            left_line: 23,
            left_text: "    qproperty!(\"gitBranches\", Read = git_branches, Constant);",
            left_markup: "<font color=\"@plain@\">&nbsp;&nbsp;&nbsp;&nbsp;</font><font color=\"@function@\"><b>qproperty!</b></font><font color=\"@plain@\">(&quot;gitBranches&quot;,&nbsp;Read&nbsp;=&nbsp;</font><font color=\"@variable@\">git_branches</font><font color=\"@plain@\">,&nbsp;Constant);</font>",
            left_kind: "removed",
            right_line: 23,
            right_text: "    qproperty!(\"diffRows\", Read = diff_rows, Constant);",
            right_markup: "<font color=\"@plain@\">&nbsp;&nbsp;&nbsp;&nbsp;</font><font color=\"@function@\"><b>qproperty!</b></font><font color=\"@plain@\">(&quot;diffRows&quot;,&nbsp;Read&nbsp;=&nbsp;</font><font color=\"@variable@\">diff_rows</font><font color=\"@plain@\">,&nbsp;Constant);</font>",
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
        aiThreadsModel.append({
            thread_id: "thread-qt-migration",
            title: "Replace the GPUI AI workspace",
            cwd: "/Volumes/hulk/dev/projects/hunk",
            workspace_label: "hunk",
            status: "active",
            active: true,
            running: true,
            created_at: 1787677200,
            updated_at: 1787677600
        })
        aiThreadsModel.append({
            thread_id: "thread-review",
            title: "Review the QtBridge state boundary",
            cwd: "/Volumes/hulk/dev/projects/hunk",
            workspace_label: "hunk",
            status: "idle",
            active: false,
            running: false,
            created_at: 1787673600,
            updated_at: 1787673900
        })
        aiTimelineModel.append({
            row_id: "item:user",
            turn_id: "turn-1",
            kind: "userMessage",
            role: "user",
            title: "You",
            text: "<b>Keep this text literal and do not parse it as HTML.</b>",
            status: "",
            streaming: false,
            mono: false,
            truncated: false,
            last_sequence: 1
        })
        aiTimelineModel.append({
            row_id: "item:assistant",
            turn_id: "turn-1",
            kind: "agentMessage",
            role: "assistant",
            title: "Assistant",
            text: "The selected thread now comes from the retained Rust reducer and renders through a bounded Qt model.",
            status: "",
            streaming: false,
            mono: false,
            truncated: false,
            last_sequence: 2
        })
        aiTimelineModel.append({
            row_id: "item:command",
            turn_id: "turn-2",
            kind: "commandExecution",
            role: "tool",
            title: "Running focused Qt tests",
            text: "nix develop -c cargo test -p hunk-qt",
            status: "streaming",
            streaming: true,
            mono: true,
            truncated: false,
            last_sequence: 3
        })
        aiTimelineModel.append({
            row_id: "turn-plan:turn-2",
            turn_id: "turn-2",
            kind: "turnPlan",
            role: "assistant",
            title: "Plan",
            text: "[x] Retain the Rust reducer\n[~] Replace the GPUI timeline",
            status: "in progress",
            streaming: true,
            mono: false,
            truncated: false,
            last_sequence: 4
        })
    }

    function openGitWorkspace() {
        shell.activateWorkspace("git")
        tryVerify(() => shell.workspaceItem !== null && shell.workspaceItem.objectName === "gitWorkspace")
    }

    function openDiffWorkspace() {
        shell.activateWorkspace("diff")
        tryVerify(() => shell.workspaceItem !== null && shell.workspaceItem.objectName === "diffWorkspace")
        shell.workspaceItem.commentsInspectorOpen = false
        shell.workspaceItem.closeCommentComposer()
    }

    function openAiWorkspace() {
        shell.activateWorkspace("ai")
        tryVerify(() => shell.workspaceItem !== null && shell.workspaceItem.objectName === "aiWorkspace")
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
        fakeBackend.diffSearchQuery = ""
        fakeBackend.diffSearchMatchCount = 0
        fakeBackend.diffSearchMatchIndex = -1
        fakeBackend.diffSearchTargetRow = -1
        fakeBackend.diffSearchMatches = []
        fakeBackend.diffCommentsReady = true
        fakeBackend.diffCommentsLoading = false
        fakeBackend.diffCommentsBusy = false
        fakeBackend.diffCommentsError = ""
        fakeBackend.diffCommentsStatusMessage = ""
        fakeBackend.diffCommentsShowNonOpen = false
        fakeBackend.diffCommentsVersion = 0
        fakeBackend.diffCommentTargetRow = -1
        fakeBackend.diffCommentTargetRevision = 0
        fakeBackend.diffCommentRecords = []
        fakeBackend.failNextDiffComment = false
        fakeBackend.gitChangedFileCount = 3
        fakeBackend.gitStagedFileCount = 1
        fakeBackend.gitUnstagedFileCount = 2
        fakeBackend.gitReady = true
        fakeBackend.gitLoading = false
        fakeBackend.gitBusy = false
        fakeBackend.gitError = ""
        fakeBackend.gitStatusMessage = "Repository refreshed"
        fakeBackend.gitActionLabel = ""
        fakeBackend.aiReady = true
        fakeBackend.aiLoading = false
        fakeBackend.aiRequiresAuthentication = false
        fakeBackend.aiConnectionState = "ready"
        fakeBackend.aiWorkspaceRoot = "/Volumes/hulk/dev/projects/hunk"
        fakeBackend.aiActiveThreadId = "thread-qt-migration"
        fakeBackend.aiActiveThreadTitle = "Replace the GPUI AI workspace"
        fakeBackend.aiActiveThreadCwd = "/Volumes/hulk/dev/projects/hunk"
        fakeBackend.aiThreadCount = 2
        fakeBackend.aiRunningThreadCount = 1
        fakeBackend.aiTimelineTotalTurnCount = 2
        fakeBackend.aiTimelineVisibleTurnCount = 2
        fakeBackend.aiTimelineHiddenTurnCount = 0
        fakeBackend.aiTimelineTotalRowCount = 4
        fakeBackend.aiTimelineHiddenRowCount = 0
        fakeBackend.aiError = ""
        fakeBackend.aiStatusMessage = "Codex thread catalog refreshed"
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
        fakeBackend.diffCommentRecords = [
            {
                comment_id: "comment-open",
                status: "open",
                file_path: "crates/hunk-qt/src/backend.rs",
                line_hint: "old 23 | new 23",
                comment_text: "Keep the Qt thread free of database and matching work.",
                clipboard_text: "[Hunk Comment]\nComment:\nKeep the Qt thread free.",
                row: 2,
                can_jump: true
            },
            {
                comment_id: "comment-resolved",
                status: "resolved",
                file_path: "crates/hunk-qt/src/backend.rs",
                line_hint: "old 22 | new 22",
                comment_text: "This model reset is now batched.",
                clipboard_text: "[Hunk Comment]\nComment:\nThis model reset is now batched.",
                row: 1,
                can_jump: true
            }
        ]
        fakeBackend.rebuildDiffComments()
        fakeBackend.lastCommand = ""
        fakeBackend.lastArgument = ""
    }

    function test_retainedWorkspaceContract() {
        compare(shell.workspaceCount, 3)
        compare(shell.workspaceIds, ["diff", "git", "ai"])
    }

    function test_aiWorkspaceUsesVirtualizedRustModelsAndPlainText() {
        openAiWorkspace()
        shell.sidebarItem.threadListView.forceLayout()
        shell.workspaceItem.timelineListView.forceLayout()

        compare(shell.sidebarItem.threadListView.count, 2)
        verify(shell.sidebarItem.threadListView.reuseItems)
        compare(shell.workspaceItem.timelineListView.count, 4)
        verify(shell.workspaceItem.timelineListView.reuseItems)

        const userRow = shell.workspaceItem.timelineListView.itemAtIndex(0)
        verify(userRow !== null)
        compare(userRow.text, "<b>Keep this text literal and do not parse it as HTML.</b>")
        compare(userRow.bodyTextItem.textFormat, TextEdit.PlainText)
    }

    function test_aiCatalogRoutesRefreshCreateAndSelectionCommands() {
        openAiWorkspace()

        shell.sidebarItem.refreshThreads()
        compare(fakeBackend.lastCommand, "refresh_ai_threads")

        shell.sidebarItem.createThread()
        compare(fakeBackend.lastCommand, "create_ai_thread")

        shell.sidebarItem.selectThread("thread-review")
        compare(fakeBackend.lastCommand, "select_ai_thread")
        compare(fakeBackend.lastArgument, "thread-review")
        compare(fakeBackend.aiActiveThreadId, "thread-review")
    }

    function test_aiArchiveRequiresConfirmationBeforeRustCommand() {
        openAiWorkspace()

        shell.sidebarItem.requestArchive(
            "thread-qt-migration",
            "Replace the GPUI AI workspace"
        )
        verify(shell.sidebarItem.archiveConfirmationVisible)
        compare(fakeBackend.lastCommand, "")

        shell.sidebarItem.confirmArchive()
        verify(!shell.sidebarItem.archiveConfirmationVisible)
        compare(fakeBackend.lastCommand, "archive_ai_thread")
        compare(fakeBackend.lastArgument, "thread-qt-migration")
        compare(fakeBackend.aiThreadCount, 1)
    }

    function test_aiTimelineRemainsVirtualizedAtItsRustBound() {
        aiTimelineModel.clear()
        for (let index = 0; index < 1000; ++index) {
            aiTimelineModel.append({
                row_id: "item:generated-" + index,
                turn_id: "turn-generated",
                kind: "agentMessage",
                role: "assistant",
                title: "Assistant",
                text: "Bounded timeline row " + index,
                status: "",
                streaming: false,
                mono: false,
                truncated: false,
                last_sequence: index
            })
        }
        fakeBackend.aiTimelineTotalRowCount = 1000

        openAiWorkspace()
        shell.workspaceItem.timelineListView.positionViewAtBeginning()
        shell.workspaceItem.timelineListView.forceLayout()
        compare(shell.workspaceItem.timelineListView.count, 1000)
        verify(shell.workspaceItem.timelineListView.itemAtIndex(0) !== null)
        verify(shell.workspaceItem.timelineListView.itemAtIndex(500) === null)
    }

    function test_aiWorkspaceStatesCoverEmptyLoadingAuthenticationAndError() {
        aiTimelineModel.clear()
        fakeBackend.aiTimelineTotalRowCount = 0
        openAiWorkspace()
        verify(shell.workspaceItem.emptyStateVisible)

        fakeBackend.aiReady = false
        fakeBackend.aiLoading = true
        verify(shell.workspaceItem.loadingStateVisible)

        fakeBackend.aiLoading = false
        fakeBackend.aiReady = true
        fakeBackend.aiRequiresAuthentication = true
        verify(shell.workspaceItem.authenticationStateVisible)

        fakeBackend.aiRequiresAuthentication = false
        fakeBackend.aiReady = false
        fakeBackend.aiError = "Codex worker disconnected"
        verify(shell.workspaceItem.errorStateVisible)
    }

    function test_aiWorkspaceRendersAtDesktopSize() {
        openAiWorkspace()
        captureSnapshot("target/hunk-qt-ai.png")
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
                left_markup: "",
                left_kind: "removed",
                right_line: index + 1,
                right_text: "let after_" + index + " = value;",
                right_markup: "",
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

    function test_diffSelectionSupportsKeyboardRangeCopyAndHunkNavigation() {
        openDiffWorkspace()
        shell.workspaceItem.resetSelection()
        shell.workspaceItem.forceActiveFocus()
        verify(shell.workspaceItem.activeFocus)

        keyClick(Qt.Key_Down)
        compare(shell.workspaceItem.selectionAnchorRow, 0)
        compare(shell.workspaceItem.selectionHeadRow, 0)

        keyClick(Qt.Key_Down, Qt.ShiftModifier)
        compare(shell.workspaceItem.selectionStart, 0)
        compare(shell.workspaceItem.selectionEnd, 1)

        keyClick(Qt.Key_A, Qt.MetaModifier)
        compare(shell.workspaceItem.selectionStart, 0)
        compare(shell.workspaceItem.selectionEnd, 2)

        shell.workspaceItem.resetSelection()
        keyClick(Qt.Key_A, Qt.ControlModifier)
        compare(shell.workspaceItem.selectionStart, 0)
        compare(shell.workspaceItem.selectionEnd, 2)

        keyClick(Qt.Key_C, Qt.MetaModifier)
        const clipboardProxy = findChild(shell.workspaceItem, "diffClipboardProxy")
        verify(clipboardProxy !== null)
        compare(clipboardProxy.text, fakeBackend.diff_selection_text(0, 2))

        keyClick(Qt.Key_F7)
        compare(shell.workspaceItem.selectionAnchorRow, 0)
        compare(shell.workspaceItem.selectionHeadRow, 0)
        keyClick(Qt.Key_F7, Qt.ShiftModifier)
        compare(shell.workspaceItem.selectionHeadRow, 0)
    }

    function test_diffSelectionSupportsPointerRangeSemantics() {
        openDiffWorkspace()
        shell.workspaceItem.diffListView.forceLayout()
        const firstCodeRow = shell.workspaceItem.diffListView.itemAtIndex(1)
        verify(firstCodeRow !== null)
        verify(findChild(firstCodeRow, "diffRowTapHandler") !== null)
        shell.workspaceItem.selectRow(1, false)
        compare(shell.workspaceItem.selectionStart, 1)
        compare(shell.workspaceItem.selectionEnd, 1)
        shell.workspaceItem.selectRow(2, true)
        compare(shell.workspaceItem.selectionStart, 1)
        compare(shell.workspaceItem.selectionEnd, 2)
    }

    function test_diffViewSwitchesBetweenSplitAndUnifiedRows() {
        openDiffWorkspace()
        shell.workspaceItem.diffListView.forceLayout()
        const changedRow = shell.workspaceItem.diffListView.itemAtIndex(2)
        verify(changedRow !== null)
        compare(changedRow.height, Theme.diffRowHeight)
        verify(changedRow.pairedChange)
        compare(changedRow.unifiedPrimaryText,
            "    qproperty!(\"gitBranches\", Read = git_branches, Constant);")

        shell.workspaceItem.setDiffMode("unified")
        shell.workspaceItem.diffListView.forceLayout()
        verify(shell.workspaceItem.unifiedMode)
        compare(changedRow.height, Theme.diffRowHeight * 2)

        shell.workspaceItem.setDiffMode("split")
        verify(!shell.workspaceItem.unifiedMode)
    }

    function test_diffSearchFindsAndNavigatesMatchingRows() {
        openDiffWorkspace()
        shell.workspaceItem.searchInput.text = "qproperty"
        shell.workspaceItem.applySearch(shell.workspaceItem.searchInput.text)
        compare(fakeBackend.diffSearchMatchCount, 2)
        compare(fakeBackend.diffSearchTargetRow, 1)
        compare(shell.workspaceItem.diffListView.currentIndex, 1)

        shell.workspaceItem.moveSearch(1)
        compare(fakeBackend.diffSearchMatchIndex, 1)
        compare(fakeBackend.diffSearchTargetRow, 2)
        compare(shell.workspaceItem.diffListView.currentIndex, 2)

        shell.workspaceItem.moveSearch(1)
        compare(fakeBackend.diffSearchMatchIndex, 0)
        compare(fakeBackend.diffSearchTargetRow, 1)

        shell.workspaceItem.searchInput.text = "missing"
        shell.workspaceItem.applySearch(shell.workspaceItem.searchInput.text)
        compare(fakeBackend.diffSearchMatchCount, 0)
        compare(fakeBackend.diffSearchTargetRow, -1)
    }

    function test_diffCommentComposerRetainsFailedDraftThenCreatesComment() {
        openDiffWorkspace()
        shell.workspaceItem.openCommentComposer(2)
        tryVerify(() => shell.workspaceItem.commentComposer !== null)
        shell.workspaceItem.commentComposer.text = "Explain why this stays off the Qt thread."

        fakeBackend.failNextDiffComment = true
        shell.workspaceItem.commentComposer.submit()
        compare(fakeBackend.lastCommand, "create_diff_comment")
        compare(shell.workspaceItem.activeCommentRow, 2)
        compare(
            shell.workspaceItem.commentComposer.text,
            "Explain why this stays off the Qt thread."
        )
        compare(fakeBackend.diffCommentsError, "Failed to save comment")

        shell.workspaceItem.commentComposer.submit()
        tryCompare(shell.workspaceItem, "activeCommentRow", -1)
        compare(fakeBackend.diffCommentsOpenCount, 2)
        verify(shell.workspaceItem.commentsInspectorOpen)
        compare(fakeBackend.diffCommentsStatusMessage, "Comment added.")
    }

    function test_diffCommentInspectorFiltersCopiesJumpsResolvesAndDeletes() {
        openDiffWorkspace()
        shell.workspaceItem.toggleCommentsInspector()
        wait(Theme.transitionDuration + 20)
        verify(shell.workspaceItem.commentsInspectorOpen)
        compare(shell.workspaceItem.commentsInspector.listView.count, 1)
        verify(shell.workspaceItem.commentsInspector.listView.reuseItems)

        const openComment = diffCommentsModel.get(0)
        shell.workspaceItem.commentsInspector.copyComment(openComment.clipboard_text)
        const clipboardProxy = findChild(shell.workspaceItem, "diffClipboardProxy")
        compare(clipboardProxy.text, openComment.clipboard_text)

        shell.workspaceItem.commentsInspector.jumpToComment("comment-open")
        compare(fakeBackend.lastCommand, "jump_to_diff_comment")
        compare(shell.workspaceItem.selectionHeadRow, 2)

        shell.workspaceItem.commentsInspector.toggleNonOpen()
        compare(fakeBackend.diffCommentsShowNonOpen, true)
        compare(shell.workspaceItem.commentsInspector.listView.count, 2)

        shell.workspaceItem.commentsInspector.setCommentStatus("comment-open", "resolved")
        compare(fakeBackend.lastCommand, "set_diff_comment_status")
        compare(fakeBackend.diffCommentsOpenCount, 0)
        compare(fakeBackend.diffCommentsResolvedCount, 2)

        shell.workspaceItem.commentsInspector.deleteComment("comment-open")
        compare(fakeBackend.lastCommand, "delete_diff_comment")
        compare(shell.workspaceItem.commentsInspector.listView.count, 1)
    }

    function test_diffCommentBadgesAndInspectorStayVirtualized() {
        const records = []
        for (let index = 0; index < 200; ++index) {
            records.push({
                comment_id: "comment-" + index,
                status: "open",
                file_path: "crates/hunk-qt/src/backend.rs",
                line_hint: "old 23 | new 23",
                comment_text: "Review note " + index,
                clipboard_text: "[Hunk Comment] " + index,
                row: 2,
                can_jump: true
            })
        }
        fakeBackend.diffCommentRecords = records
        fakeBackend.rebuildDiffComments()

        openDiffWorkspace()
        shell.workspaceItem.diffListView.positionViewAtIndex(2, ListView.Contain)
        shell.workspaceItem.diffListView.forceLayout()
        const changedRow = shell.workspaceItem.diffListView.itemAtIndex(2)
        verify(changedRow !== null)
        compare(changedRow.commentCount, 200)
        const badge = findChild(changedRow, "diffCommentBadge")
        verify(badge !== null)
        compare(badge.width, 28)

        shell.workspaceItem.toggleCommentsInspector()
        wait(Theme.transitionDuration + 20)
        shell.workspaceItem.commentsInspector.listView.forceLayout()
        compare(shell.workspaceItem.commentsInspector.listView.count, 64)
        verify(shell.workspaceItem.commentsInspector.listView.itemAtIndex(0) !== null)
        verify(shell.workspaceItem.commentsInspector.listView.itemAtIndex(50) === null)
    }

    function test_diffCommentsRenderAtDesktopSize() {
        openDiffWorkspace()
        shell.workspaceItem.toggleCommentsInspector()
        shell.workspaceItem.openCommentComposer(2)
        tryVerify(() => shell.workspaceItem.commentComposer !== null)
        shell.workspaceItem.commentComposer.text = "Clarify the epoch boundary before merging."
        wait(Theme.transitionDuration + 20)
        captureSnapshot("target/hunk-qt-diff-comments.png")
    }

    function test_diffWorkspaceRendersAtDesktopSize() {
        openDiffWorkspace()
        shell.workspaceItem.searchInput.text = ""
        shell.workspaceItem.applySearch("")
        shell.workspaceItem.selectRow(2, false)
        captureSnapshot("target/hunk-qt-diff.png")
        shell.workspaceItem.resetSelection()
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
