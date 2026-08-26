import QtQuick
import QtTest
import Hunk 1.0
TestCase {
    name: "HunkShell"
    width: 1280
    height: 760
    when: windowShown
    property bool snapshotReady: false
    property bool snapshotSaved: false
    ListModel { id: compareSourcesModel }
    ListModel { id: diffFilesModel }
    ListModel { id: diffRowsModel }
    ListModel { id: diffCommentsModel }
    ListModel { id: aiThreadsModel }
    ListModel { id: aiTimelineModel }
    ListModel { id: aiAttachmentsModel }
    ListModel { id: terminalTabsModel } ListModel { id: terminalRowsModel } ListModel { id: browserTabsModel; ListElement { tab_id: "tab-1"; title: "New tab"; url: "about:blank"; loading: false } } FakeBrowser { id: fakeBrowser; tabs: browserTabsModel } FakeUpdater { id: fakeUpdates }
    ListModel { id: aiModelsModel; ListElement { value: ""; label: "Server default" } ListElement { value: "gpt-5.5"; label: "GPT-5.5" } }
    ListModel { id: aiEffortsModel; ListElement { value: ""; label: "Model default" } ListElement { value: "high"; label: "High" } }
    ListModel { id: aiServiceTiersModel; ListElement { value: "standard"; label: "Standard" } ListElement { value: "fast"; label: "Fast" } ListElement { value: "flex"; label: "Flex" } }
    QtObject {
        id: fakeBackend
        property string activeWorkspace: "diff"
        property bool ready: true
        property string statusMessage: "Test backend ready"; property QtObject browser: fakeBrowser; property QtObject updates: fakeUpdates
        property string lastRequestedWorkspace: ""
        property var diffFiles: diffFilesModel
        property var diffRows: diffRowsModel
        property var diffCompareSources: compareSourcesModel
        property string diffCompareLeftLabel: "migration/05-qt-git"
        property string diffCompareRightLabel: "Primary Checkout"
        property int diffCompareLeftIndex: 1
        property int diffCompareRightIndex: 0
        property int diffCompareFileCount: 3
        property string diffSelectedPath: "crates/hunk-desktop/src/backend.rs"
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
        property string gitRoot: "/Volumes/hulk/dev/projects/hunk"
        property string gitRepositoryName: "hunk"
        property string aiProjectCatalogJson: JSON.stringify([
            { project_path: "/Users/test/Documents/glab", name: "glab" },
            { project_path: "/Users/test/Documents/lightning-service-rust", name: "lightning-service-rust" },
            { project_path: "/Volumes/hulk/dev/projects/hunk", name: "hunk" }
        ])
        property string gitBranchName: "migration/05-qt-git"
        property int gitChangedFileCount: 3
        property var terminalTabs: terminalTabsModel; property var terminalRows: terminalRowsModel
        property bool terminalOpen: false; property int terminalActiveTabId: 1; property int terminalActiveTabIndex: 0; property string terminalShellLabel: "zsh"
        property string terminalStatus: "idle"; property string terminalStatusMessage: ""; property string terminalCwd: gitRoot
        property int terminalDisplayOffset: 0; property bool terminalMouseMode: false; property int terminalCursorRow: -1; property int terminalCursorColumn: -1; property string terminalCursorShape: "hidden"; property bool terminalCursorVisible: false; property int terminalScreenRevision: 0; property int terminalFocusRevision: 0
        property var aiThreads: aiThreadsModel
        property var aiTimeline: aiTimelineModel
        property var aiAttachments: aiAttachmentsModel; readonly property int aiAttachmentCount: aiAttachmentsModel.count; property bool aiAttachmentPending: false; property bool aiModelSupportsImageInputs: true
        property QtObject aiModels: aiModelsModel; property QtObject aiEfforts: aiEffortsModel; property QtObject aiServiceTiers: aiServiceTiersModel
        property int aiSelectedModelIndex: 1; property int aiSelectedEffortIndex: 1; property int aiSelectedServiceTierIndex: 0; property int aiEffortOptionCount: 2
        property string aiSelectedModelLabel: "GPT-5.5"; property string aiSelectedEffortLabel: "High"; property string aiSelectedCollaborationMode: "code"; property string aiSelectedCollaborationLabel: "Code"; property string aiSelectedServiceTierLabel: "Standard"; property string aiApprovalPolicyLabel: "Full access"
        property bool aiMadMaxMode: true; readonly property bool aiSessionControlsLocked: aiTurnRunning; property bool aiContextAvailable: true; property int aiContextPercentUsed: 27; property int aiContextPercentLeft: 73
        property string aiContextTokenSummary: "73k / 258k tokens"; property string aiContextInputTokens: "3,600"; property string aiContextCachedInputTokens: "900"; property string aiContextOutputTokens: "700"; property string aiContextReasoningTokens: "300"; property string aiContextBillableTokens: "4,300"
        property bool aiReady: true
        property bool aiLoading: false
        property bool aiRequiresAuthentication: false
        property string aiAccountSummary: "ChatGPT: test@example.com (Plus)"; property bool aiAccountConnected: true; property bool aiLoginPending: false
        property int aiApprovalRequestCount: 0; property int aiInputRequestCount: 0
        property bool aiFiveHourLimitAvailable: true; property int aiFiveHourLimitRemainingPercent: 75; property string aiFiveHourLimitResetLabel: "Resets in 2h"
        property bool aiWeeklyLimitAvailable: true; property int aiWeeklyLimitRemainingPercent: 50; property string aiWeeklyLimitResetLabel: "Resets in 4d"
        property string aiConnectionState: "ready"
        property string aiWorkspaceRoot: "/Volumes/hulk/dev/projects/hunk"
        property string aiActiveThreadId: "thread-qt-migration"
        property string aiActiveThreadTitle: "Replace the GPUI AI workspace"
        property string aiActiveThreadCwd: "/Volumes/hulk/dev/projects/hunk"
        property string aiActiveTurnId: "turn-2"
        property bool aiTurnRunning: true
        property bool aiThreadActionPending: false
        property bool aiPromptPending: false
        property int aiPromptAcceptedRevision: 0
        property int aiQueuedMessageCount: 0
        property int aiActiveQueuedMessageCount: 0
        property int nextAiQueuedMessageId: 0
        property bool aiActiveQueueSending: false
        property bool aiInterruptPending: false
        property int aiPendingRequestCount: 0
        property int aiActiveRequestCount: 0
        property string aiRequestId: ""
        property string aiRequestKind: ""
        property string aiRequestTitle: ""
        property string aiRequestDescription: ""
        property string aiRequestReason: ""
        property string aiRequestQuestionsJson: "[]"
        property bool aiRequestAnswerable: false
        property bool aiRequestResolving: false
        property int aiThreadCount: 2
        property int aiRunningThreadCount: 1
        property int aiTimelineTotalTurnCount: 2
        property int aiTimelineVisibleTurnCount: 2
        property int aiTimelineHiddenTurnCount: 0
        property int aiTimelineTotalRowCount: 4
        property int aiTimelineHiddenRowCount: 0
        property string aiError: ""
        property string aiStatusMessage: "Codex thread catalog refreshed"
        property string lastCommand: ""
        property string lastArgument: ""
        property int commandCount: 0
        property string lastAnswersJson: ""
        property bool failNextAiRequest: false
        property var pendingAiRequestIds: []
        property var recoveredAiPrompts: ({})
        property var recoveredAiAttachmentPaths: ({})
        signal diffCommentsStateChanged
        signal aiStateChanged
        signal aiSessionStateChanged; signal terminalStateChanged; signal terminalScreenChanged; signal terminalFocusChanged
        function record(command, argument) {
            commandCount += 1
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
        function refresh_diff_compare() { record("refresh_diff_compare") }
        function select_diff_compare_source(side, index) {
            const source = compareSourcesModel.get(index)
            if (side === "left") {
                diffCompareLeftIndex = index
                diffCompareLeftLabel = source.label
            } else {
                diffCompareRightIndex = index
                diffCompareRightLabel = source.label
            }
            record("select_diff_compare_" + side, String(index))
        }
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
        function select_git_root(root) {
            record("select_root", root)
            gitRoot = root
            gitRepositoryName = root.slice(root.lastIndexOf("/") + 1)
        }
        function set_terminal_open(open) { terminalOpen = open; if (open) { terminalFocusRevision += 1; terminalFocusChanged() } terminalStateChanged(); return true }
        function new_terminal_tab() { return true } function close_terminal_tab(tabId) { return tabId > 0 }
        function select_terminal_tab(tabId) { terminalActiveTabId = tabId; terminalStateChanged(); return true } function move_terminal_tab(direction) { return direction !== 0 }
        function resize_terminal(rows, cols) { return rows > 0 && cols > 0 } function send_terminal_key() { return true } function write_terminal_text() { return true } function paste_terminal_text() { return true }
        function report_terminal_focus() { return true } function terminal_pointer_button() { return true } function terminal_pointer_move() { return true } function terminal_wheel() { return true }
        function clear_terminal_screen() { return true } function scroll_terminal() { return true } function terminal_selection_text() { return "" } function run_terminal_command() { terminalOpen = true; terminalFocusRevision += 1; terminalFocusChanged(); terminalStateChanged(); return true }
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
                    aiTurnRunning = aiThreadsModel.get(index).running
                    aiActiveTurnId = aiTurnRunning ? "turn-for-" + threadId : ""
                }
            }
            aiStateChanged()
        }
        function create_ai_thread() { record("create_ai_thread") }
        function fork_ai_thread() {
            if (!aiReady || aiLoading || aiRequiresAuthentication
                    || aiActiveThreadId.length === 0 || aiTurnRunning
                    || aiThreadActionPending || aiPromptPending
                    || aiInterruptPending || aiRequestId.length > 0
                    || aiRequestResolving) {
                return false
            }
            record("fork_ai_thread", aiActiveThreadId)
            aiThreadActionPending = true
            aiStateChanged()
            return true
        }
        function archive_ai_thread(threadId) {
            for (let index = 0; index < aiThreadsModel.count; ++index) {
                const thread = aiThreadsModel.get(index)
                if (thread.thread_id === threadId && thread.attention)
                    return
            }
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
                aiActiveTurnId = ""
                aiTurnRunning = false
                aiTimelineModel.clear()
                aiTimelineTotalRowCount = 0
            }
            aiStateChanged()
        }
        function toggle_ai_thread_bookmark(threadId) {
            for (let index = 0; index < aiThreadsModel.count; ++index) {
                const thread = aiThreadsModel.get(index)
                if (thread.thread_id !== threadId)
                    continue
                const bookmarked = !thread.bookmarked
                aiThreadsModel.setProperty(index, "bookmarked", bookmarked)
                record("toggle_ai_thread_bookmark", threadId)
                if (bookmarked) {
                    aiThreadsModel.move(index, 0, 1)
                } else {
                    let destination = 0
                    while (destination + 1 < aiThreadsModel.count
                            && aiThreadsModel.get(destination + 1).bookmarked)
                        destination += 1
                    while (destination + 1 < aiThreadsModel.count
                            && aiThreadsModel.get(destination + 1).created_at
                                > thread.created_at)
                        destination += 1
                    aiThreadsModel.move(index, destination, 1)
                }
                aiStateChanged()
                return true
            }
            return false
        }
        function select_ai_model(index) { record("select_ai_model", String(index)); return true }
        function select_ai_effort(index) { record("select_ai_effort", String(index)); return true }
        function select_ai_collaboration_mode(mode) { record("select_ai_collaboration_mode", mode); return true }
        function select_ai_service_tier(index) { record("select_ai_service_tier", String(index)); return true }
        function set_ai_mad_max_mode(enabled) { record("set_ai_mad_max_mode", String(enabled)); return true }
        function start_ai_chatgpt_login() { record("start_ai_chatgpt_login"); return true }
        function cancel_ai_chatgpt_login() { record("cancel_ai_chatgpt_login"); return true }
        function logout_ai_account() { record("logout_ai_account"); return true }
        function add_ai_attachments(pathsJson) {
            const paths = JSON.parse(pathsJson)
            for (const path of paths) {
                const parts = path.split("/")
                aiAttachmentsModel.append({ path: path, display_name: parts[parts.length - 1] })
            }
            record("add_ai_attachments", pathsJson)
            aiStateChanged()
            return paths.length > 0
        }
        function remove_ai_attachment(index) {
            if (index < 0 || index >= aiAttachmentsModel.count)
                return false
            aiAttachmentsModel.remove(index)
            aiStateChanged()
            return true
        }
        function send_ai_prompt(prompt) {
            if (!aiReady || aiLoading || aiRequiresAuthentication
                    || aiActiveThreadId.length === 0 || aiPromptPending
                    || aiInterruptPending || aiRequestId.length > 0
                    || aiRequestResolving
                    || (prompt.trim().length === 0 && aiAttachmentCount === 0)
                    || (aiAttachmentCount > 0 && !aiModelSupportsImageInputs))
                return false
            record(aiTurnRunning ? "steer_ai_prompt" : "send_ai_prompt", prompt)
            aiPromptPending = true
            aiStateChanged()
            return true
        }
        function queue_ai_follow_up(prompt) {
            const text = prompt.trim()
            if (!aiReady || aiLoading || aiRequiresAuthentication
                    || !aiTurnRunning || aiPromptPending || aiInterruptPending
                    || aiRequestId.length > 0 || aiRequestResolving
                    || (text.length === 0 && aiAttachmentCount === 0)
                    || (aiAttachmentCount > 0 && !aiModelSupportsImageInputs)
                    || aiQueuedMessageCount >= 64)
                return false
            record("queue_ai_follow_up", text)
            nextAiQueuedMessageId += 1
            aiTimelineModel.append({
                row_id: "queued-message:" + nextAiQueuedMessageId,
                turn_id: "",
                kind: "queuedMessage",
                role: "user",
                title: "You",
                text: text,
                command: "", cwd: "",
                status: "queued",
                streaming: false,
                mono: false,
                truncated: false,
                last_sequence: 0
            })
            aiQueuedMessageCount += 1
            aiActiveQueuedMessageCount += 1
            aiAttachmentsModel.clear()
            aiStateChanged()
            return true
        }
        function first_queued_ai_message_index() {
            for (let index = 0; index < aiTimelineModel.count; ++index) {
                if (aiTimelineModel.get(index).kind === "queuedMessage")
                    return index
            }
            return -1
        }
        function edit_last_ai_queued_prompt() {
            for (let index = aiTimelineModel.count - 1; index >= 0; --index) {
                const queued = aiTimelineModel.get(index)
                if (queued.kind === "queuedMessage" && queued.status === "queued") {
                    const prompt = queued.text
                    aiTimelineModel.remove(index)
                    aiQueuedMessageCount -= 1
                    aiActiveQueuedMessageCount -= 1
                    record("edit_last_ai_queued_prompt", prompt)
                    aiStateChanged()
                    return prompt
                }
            }
            return ""
        }
        function take_ai_recovered_prompt(threadId) {
            const prompt = recoveredAiPrompts[threadId] || ""
            const paths = recoveredAiAttachmentPaths[threadId] || []
            delete recoveredAiPrompts[threadId]
            delete recoveredAiAttachmentPaths[threadId]
            for (const path of paths)
                aiAttachmentsModel.append({ path: path, display_name: path })
            return prompt
        }
        function finish_ai_turn() {
            aiTurnRunning = false
            aiActiveTurnId = ""
            const queuedIndex = first_queued_ai_message_index()
            if (queuedIndex >= 0) {
                aiTimelineModel.setProperty(queuedIndex, "status", "sending")
                aiTimelineModel.setProperty(queuedIndex, "streaming", true)
                aiActiveQueueSending = true
                record("send_queued_ai_prompt", aiTimelineModel.get(queuedIndex).text)
            }
            aiStateChanged()
        }
        function accept_queued_ai_prompt() {
            const queuedIndex = first_queued_ai_message_index()
            if (queuedIndex < 0)
                return
            aiTimelineModel.remove(queuedIndex)
            aiQueuedMessageCount -= 1
            aiActiveQueuedMessageCount -= 1
            aiActiveQueueSending = false
            aiStateChanged()
        }
        function accept_ai_prompt() {
            aiPromptPending = false
            aiPromptAcceptedRevision += 1
            aiAttachmentsModel.clear()
            aiStateChanged()
        }
        function fail_ai_prompt() {
            aiPromptPending = false
            aiError = "Codex rejected the message"
            aiStateChanged()
        }
        function interrupt_ai_turn() {
            if (!aiTurnRunning || aiThreadActionPending || aiPromptPending
                    || aiInterruptPending)
                return false
            record("interrupt_ai_turn", aiActiveTurnId)
            aiInterruptPending = true
            aiStateChanged()
            return true
        }
        function complete_ai_interrupt() {
            aiInterruptPending = false
            aiTurnRunning = false
            aiActiveTurnId = ""
            if (aiActiveQueuedMessageCount > 0) {
                const prompts = []
                for (let index = aiTimelineModel.count - 1; index >= 0; --index) {
                    const item = aiTimelineModel.get(index)
                    if (item.kind === "queuedMessage") {
                        prompts.unshift(item.text)
                        aiTimelineModel.remove(index)
                    }
                }
                recoveredAiPrompts[aiActiveThreadId] = prompts.join("\n\n")
                aiQueuedMessageCount = 0
                aiActiveQueuedMessageCount = 0
                aiActiveQueueSending = false
            }
            aiStateChanged()
        }
        function set_ai_attention(threadId, attention) {
            for (let index = 0; index < aiThreadsModel.count; ++index) {
                if (aiThreadsModel.get(index).thread_id === threadId)
                    aiThreadsModel.setProperty(index, "attention", attention)
            }
        }
        function show_ai_approval(requestId) {
            if (!pendingAiRequestIds.includes(requestId))
                pendingAiRequestIds = pendingAiRequestIds.concat([requestId])
            aiPendingRequestCount = pendingAiRequestIds.length
            aiActiveRequestCount = 1
            aiRequestId = requestId
            aiRequestKind = "approval"
            aiRequestTitle = "Command execution approval"
            aiRequestDescription = "Command: cargo test -p hunk-desktop"
            aiRequestReason = "Codex needs permission to run the focused tests."
            aiRequestQuestionsJson = "[]"
            aiRequestAnswerable = true
            aiRequestResolving = false
            set_ai_attention(aiActiveThreadId, true)
            aiStateChanged()
        }
        function show_ai_user_input(requestId, questionsJson) {
            if (!pendingAiRequestIds.includes(requestId))
                pendingAiRequestIds = pendingAiRequestIds.concat([requestId])
            aiPendingRequestCount = pendingAiRequestIds.length
            aiActiveRequestCount = 1
            aiRequestId = requestId
            aiRequestKind = "userInput"
            aiRequestTitle = "Codex needs your input"
            aiRequestDescription = "Answer the questions below so the active turn can continue."
            aiRequestReason = ""
            aiRequestQuestionsJson = questionsJson
            aiRequestAnswerable = true
            aiRequestResolving = false
            set_ai_attention(aiActiveThreadId, true)
            aiStateChanged()
        }
        function resolve_ai_approval(requestId, accept) {
            if (requestId !== aiRequestId || aiRequestKind !== "approval"
                    || aiRequestResolving)
                return false
            record(accept ? "accept_ai_approval" : "decline_ai_approval", requestId)
            aiRequestResolving = true
            aiStateChanged()
            return true
        }
        function submit_ai_user_input(requestId, answersJson) {
            if (requestId !== aiRequestId || aiRequestKind !== "userInput"
                    || aiRequestResolving)
                return false
            lastAnswersJson = answersJson
            if (failNextAiRequest) {
                failNextAiRequest = false
                aiError = "Codex input response failed"
                aiStateChanged()
                return false
            }
            record("submit_ai_user_input", requestId)
            aiRequestResolving = true
            aiStateChanged()
            return true
        }
        function ai_request_pending(requestId) {
            return pendingAiRequestIds.includes(requestId)
        }
        function complete_ai_request() {
            const threadId = aiActiveThreadId
            pendingAiRequestIds = pendingAiRequestIds.filter(
                requestId => requestId !== aiRequestId)
            aiPendingRequestCount = pendingAiRequestIds.length
            aiActiveRequestCount = 0
            aiRequestId = ""
            aiRequestKind = ""
            aiRequestTitle = ""
            aiRequestDescription = ""
            aiRequestReason = ""
            aiRequestQuestionsJson = "[]"
            aiRequestAnswerable = false
            aiRequestResolving = false
            set_ai_attention(threadId, false)
            aiStateChanged()
        }
    }
    Shell {
        id: shell
        width: 1280
        height: 760
        backend: fakeBackend
    }
    function populateModels() {
        compareSourcesModel.clear()
        diffFilesModel.clear()
        diffRowsModel.clear()
        diffCommentsModel.clear()
        aiThreadsModel.clear()
        aiTimelineModel.clear()
        diffFilesModel.append({
            path: "crates/hunk-desktop/src/backend.rs",
            file_name: "backend.rs",
            directory: "crates/hunk-desktop/src",
            status_tag: "M",
            additions: 132,
            removals: 8
        })
        diffFilesModel.append({
            path: "crates/hunk-desktop/src/qml/Hunk/DiffWorkspace.qml",
            file_name: "DiffWorkspace.qml",
            directory: "crates/hunk-desktop/src/qml/Hunk",
            status_tag: "A",
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
        compareSourcesModel.append({
            source_id: "workspace:primary",
            label: "Primary Checkout",
            detail: "Primary checkout · master",
            kind: "workspace",
            branch_name: "master",
            target_id: "primary",
            root: "/Volumes/hulk/dev/projects/hunk"
        })
        compareSourcesModel.append({
            source_id: "branch:migration/05-qt-git",
            label: "migration/05-qt-git",
            detail: "Local branch · checked out",
            kind: "branch",
            branch_name: "migration/05-qt-git",
            target_id: "",
            root: ""
        })
        compareSourcesModel.append({
            source_id: "branch:master",
            label: "master",
            detail: "Local branch",
            kind: "branch",
            branch_name: "master",
            target_id: "",
            root: ""
        })
        aiThreadsModel.append({
            thread_id: "thread-qt-migration",
            title: "Replace the GPUI AI workspace",
            cwd: "/Volumes/hulk/dev/projects/hunk",
            workspace_label: "hunk",
            status: "active",
            active: true,
            running: true,
            attention: false,
            bookmarked: false,
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
            attention: false,
            bookmarked: false,
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
            command: "", cwd: "",
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
            command: "", cwd: "",
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
            text: "nix develop -c cargo test -p hunk-desktop",
            command: "nix develop -c cargo test -p hunk-desktop", cwd: "/Volumes/hulk/dev/projects/hunk",
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
            command: "", cwd: "",
            status: "in progress",
            streaming: true,
            mono: false,
            truncated: false,
            last_sequence: 4
        })
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
    function verifyTerminalFocusRoundTrip(item) {
        item.forceActiveFocus()
        tryCompare(item, "activeFocus", true)
        shell.setTerminalOpen(true)
        tryVerify(() => findChild(shell, "terminalDrawer").item !== null)
        shell.setTerminalOpen(false)
        tryVerify(() => findChild(shell, "terminalDrawer").item === null)
        tryCompare(item, "activeFocus", true)
    }
    function init() {
        shell.width = 1280
        fakeBackend.activeWorkspace = "diff"
        wait(0)
        fakeBackend.lastRequestedWorkspace = ""
        fakeBackend.lastCommand = ""
        fakeBackend.lastArgument = ""
        fakeBackend.commandCount = 0
        fakeBackend.diffSelectedPath = "crates/hunk-desktop/src/backend.rs"
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
        fakeBackend.gitRoot = "/Volumes/hulk/dev/projects/hunk"
        fakeBackend.gitRepositoryName = "hunk"
        fakeBackend.diffCompareLeftLabel = "migration/05-qt-git"
        fakeBackend.diffCompareRightLabel = "Primary Checkout"
        fakeBackend.diffCompareLeftIndex = 1
        fakeBackend.diffCompareRightIndex = 0
        fakeBackend.diffCompareFileCount = 3
        fakeBackend.terminalOpen = false
        terminalTabsModel.clear(); terminalRowsModel.clear()
        fakeBackend.aiReady = true
        fakeBackend.aiLoading = false
        fakeBackend.aiRequiresAuthentication = false
        fakeBackend.aiConnectionState = "ready"
        fakeBackend.aiWorkspaceRoot = "/Volumes/hulk/dev/projects/hunk"
        fakeBackend.aiActiveThreadId = "thread-qt-migration"
        fakeBackend.aiActiveThreadTitle = "Replace the GPUI AI workspace"
        fakeBackend.aiActiveThreadCwd = "/Volumes/hulk/dev/projects/hunk"
        fakeBackend.aiActiveTurnId = "turn-2"
        fakeBackend.aiTurnRunning = true
        fakeBackend.aiThreadActionPending = false
        fakeBackend.aiPromptPending = false
        fakeBackend.aiAttachmentPending = false
        fakeBackend.aiPromptAcceptedRevision = 0
        fakeBackend.aiQueuedMessageCount = 0
        fakeBackend.aiActiveQueuedMessageCount = 0
        fakeBackend.nextAiQueuedMessageId = 0
        fakeBackend.aiActiveQueueSending = false
        fakeBackend.aiInterruptPending = false
        fakeBackend.aiPendingRequestCount = 0
        fakeBackend.aiActiveRequestCount = 0
        fakeBackend.aiRequestId = ""
        fakeBackend.aiRequestKind = ""
        fakeBackend.aiRequestTitle = ""
        fakeBackend.aiRequestDescription = ""
        fakeBackend.aiRequestReason = ""
        fakeBackend.aiRequestQuestionsJson = "[]"
        fakeBackend.aiRequestAnswerable = false
        fakeBackend.aiRequestResolving = false
        fakeBackend.aiThreadCount = 2
        fakeBackend.aiRunningThreadCount = 1
        fakeBackend.aiTimelineTotalTurnCount = 2
        fakeBackend.aiTimelineVisibleTurnCount = 2
        fakeBackend.aiTimelineHiddenTurnCount = 0
        fakeBackend.aiTimelineTotalRowCount = 4
        fakeBackend.aiTimelineHiddenRowCount = 0
        fakeBackend.aiError = ""
        fakeBackend.aiStatusMessage = "Codex thread catalog refreshed"
        fakeBackend.aiModelSupportsImageInputs = true
        aiAttachmentsModel.clear()
        fakeBackend.lastAnswersJson = ""
        fakeBackend.failNextAiRequest = false
        fakeBackend.pendingAiRequestIds = []
        fakeBackend.recoveredAiPrompts = ({})
        fakeBackend.recoveredAiAttachmentPaths = ({})
        shell.aiDraftWorkspaceRoot = fakeBackend.aiWorkspaceRoot
        shell.aiDraftStore = ({})
        shell.aiRequestAnswerStore = ({})
        snapshotReady = false
        snapshotSaved = false
        populateModels()
        fakeBackend.diffCommentRecords = [
            {
                comment_id: "comment-open",
                status: "open",
                file_path: "crates/hunk-desktop/src/backend.rs",
                line_hint: "old 23 | new 23",
                comment_text: "Keep the Qt thread free of database and matching work.",
                clipboard_text: "[Hunk Comment]\nComment:\nKeep the Qt thread free.",
                row: 2,
                can_jump: true
            },
            {
                comment_id: "comment-resolved",
                status: "resolved",
                file_path: "crates/hunk-desktop/src/backend.rs",
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
        compare(shell.workspaceCount, 2)
        compare(shell.workspaceIds, ["diff", "ai"])
    }
    function test_diffCompareSourcesRouteThroughBackend() {
        openDiffWorkspace()
        compare(shell.workspaceItem.leftComparePicker.currentLabel, "migration/05-qt-git")
        compare(shell.workspaceItem.rightComparePicker.currentLabel, "Primary Checkout")
        shell.workspaceItem.leftComparePicker.selected(2)
        compare(fakeBackend.lastCommand, "select_diff_compare_left")
        compare(fakeBackend.lastArgument, "2")
        compare(fakeBackend.diffCompareLeftLabel, "master")

        fakeBackend.lastCommand = ""
        shell.workspaceItem.refreshButton.clicked()
        compare(fakeBackend.lastCommand, "refresh_diff_compare")

        fakeBackend.diffCompareLeftIndex = -1
        fakeBackend.diffCompareRightIndex = -1
        wait(0)
        shell.workspaceItem.refreshButton.clicked()
        compare(fakeBackend.lastCommand, "refresh")
    }
    function test_diffComparePopupActivatesFocusedSourceFromKeyboard() {
        openDiffWorkspace()
        const picker = shell.workspaceItem.leftComparePicker
        picker.popup.open()
        tryCompare(picker.popup, "opened", true)
        picker.listView.currentIndex = 2
        picker.listView.forceActiveFocus()
        tryCompare(picker.listView, "activeFocus", true)
        keyClick(Qt.Key_Return)
        tryCompare(picker.popup, "opened", false)
        compare(fakeBackend.lastCommand, "select_diff_compare_left")
        compare(fakeBackend.lastArgument, "2")
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
    function test_aiProjectCatalogRoutesRememberedProjectSelection() {
        fakeBackend.aiTurnRunning = false
        openAiWorkspace()
        const projectList = shell.sidebarItem.projectListView
        projectList.forceLayout()
        compare(projectList.count, 3)
        compare(projectList.itemAtIndex(2).active, true)

        shell.sidebarItem.selectProject("/Users/test/Documents/glab")

        compare(fakeBackend.lastCommand, "select_root")
        compare(fakeBackend.lastArgument, "/Users/test/Documents/glab")
        compare(fakeBackend.gitRoot, "/Users/test/Documents/glab")
        tryCompare(projectList.itemAtIndex(0), "active", true)

        const lightningRow = projectList.itemAtIndex(1)
        verify(lightningRow.activeFocusOnTab)
        lightningRow.forceActiveFocus()
        keyClick(Qt.Key_Return)
        compare(fakeBackend.lastArgument,
            "/Users/test/Documents/lightning-service-rust")
        compare(fakeBackend.gitRoot,
            "/Users/test/Documents/lightning-service-rust")
    }
    function test_aiBookmarksReorderAndRemainActionable() {
        openAiWorkspace()
        const threadList = shell.sidebarItem.threadListView
        threadList.forceLayout()
        const reviewRow = threadList.itemAtIndex(1)
        verify(reviewRow !== null)
        verify(reviewRow.bookmarkButton.enabled)
        reviewRow.bookmarkButton.clicked()
        tryCompare(fakeBackend, "lastCommand", "toggle_ai_thread_bookmark")
        tryCompare(aiThreadsModel.get(0), "thread_id", "thread-review")
        tryCompare(aiThreadsModel.get(0), "bookmarked", true)
        threadList.forceLayout()
        const bookmarkedRow = threadList.itemAtIndex(0)
        verify(bookmarkedRow !== null)
        tryCompare(bookmarkedRow.bookmarkButton, "label", "★")
        tryCompare(bookmarkedRow.bookmarkButton, "accessibleName", "Remove bookmark")
        const commandCount = fakeBackend.commandCount
        bookmarkedRow.bookmarkButton.clicked()
        tryCompare(fakeBackend, "commandCount", commandCount + 1)
        tryCompare(aiThreadsModel.get(0), "thread_id", "thread-qt-migration")
        tryCompare(aiThreadsModel.get(1), "bookmarked", false)
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
    function test_aiForkRequiresAnIdleThreadAndDeduplicatesUntilCompletion() {
        openAiWorkspace()
        const workspace = shell.workspaceItem
        verify(!workspace.forkButton.enabled)
        verify(workspace.composer.stopButton.enabled)
        fakeBackend.aiThreadActionPending = true
        fakeBackend.aiStateChanged()
        verify(!workspace.composer.stopButton.enabled)
        fakeBackend.aiThreadActionPending = false
        fakeBackend.aiStateChanged()
        fakeBackend.aiTurnRunning = false
        fakeBackend.aiActiveTurnId = ""
        fakeBackend.aiStateChanged()
        fakeBackend.aiReady = false
        fakeBackend.aiStateChanged()
        verify(!workspace.forkButton.enabled)
        fakeBackend.aiReady = true
        fakeBackend.aiStateChanged()
        tryVerify(() => workspace.forkButton.enabled)
        verify(workspace.forkThread())
        compare(fakeBackend.lastCommand, "fork_ai_thread")
        compare(fakeBackend.lastArgument, "thread-qt-migration")
        verify(fakeBackend.aiThreadActionPending)
        verify(!workspace.forkButton.enabled)
        verify(!workspace.composer.editor.enabled)
        fakeBackend.show_ai_approval("approval-during-fork")
        tryCompare(workspace.requestPanel, "loadedRequestId", "approval-during-fork")
        verify(shell.sidebarItem.commandPending)
        verify(!workspace.requestPanel.acceptButton.enabled)
        compare(workspace.requestPanel.acceptButton.label, "Accept")
        const commandCount = fakeBackend.commandCount
        verify(!workspace.forkThread())
        compare(fakeBackend.commandCount, commandCount)
        fakeBackend.aiThreadActionPending = false
        fakeBackend.aiStateChanged()
        tryVerify(() => workspace.requestPanel.acceptButton.enabled
            && workspace.requestPanel.acceptButton.activeFocus)
        fakeBackend.complete_ai_request()
        tryVerify(() => workspace.composer.editor.enabled
            && workspace.composer.editor.activeFocus
            && workspace.forkButton.enabled
            && !shell.sidebarItem.commandPending)
    }
    function test_aiArchiveConfirmationClosesWhenARequestArrives() {
        openAiWorkspace()
        shell.sidebarItem.requestArchive("thread-review", "Review thread")
        verify(shell.sidebarItem.archiveConfirmationVisible)
        fakeBackend.show_ai_approval("approval-cancels-archive")
        tryVerify(() => !shell.sidebarItem.archiveConfirmationVisible)
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
                text: "Bounded timeline row " + index, command: "", cwd: "",
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
        fakeBackend.aiReady = true
        fakeBackend.aiRequiresAuthentication = true
        verify(shell.workspaceItem.loadingStateVisible)
        verify(!shell.workspaceItem.authenticationStateVisible)
        verify(!shell.workspaceItem.emptyStateVisible)
        verify(shell.sidebarItem.loadingStateVisible)
        verify(!shell.sidebarItem.threadListStateVisible)
        fakeBackend.aiLoading = false
        fakeBackend.aiReady = true
        verify(shell.workspaceItem.authenticationStateVisible)
        verify(shell.sidebarItem.threadListStateVisible)
        fakeBackend.aiRequiresAuthentication = false
        fakeBackend.aiReady = false
        fakeBackend.aiError = "Codex worker disconnected"
        verify(shell.workspaceItem.errorStateVisible)
    }
    function test_aiWorkspaceRendersAtDesktopSize() {
        openAiWorkspace()
        captureSnapshot("target/hunk-desktop-ai.png")
    }
    function test_aiConversationUsesABoundedCenteredColumn() {
        openAiWorkspace()
        const workspace = shell.workspaceItem
        const timeline = workspace.timelineListView
        const composer = workspace.composer
        const requestPanel = workspace.requestPanel
        compare(workspace.conversationWidth, 680)
        compare(timeline.width, workspace.conversationWidth)
        compare(composer.width, workspace.conversationWidth)
        compare(requestPanel.width, workspace.conversationWidth)
        compare(Math.round(composer.x + composer.width / 2),
            Math.round(workspace.width / 2))
        compare(Math.round(timeline.x + timeline.width / 2),
            Math.round(workspace.width / 2))

        shell.width = 720
        wait(0)
        compare(workspace.conversationWidth, workspace.width - 32)
        compare(composer.width, workspace.conversationWidth)
        compare(timeline.width, workspace.conversationWidth)
    }
    function test_aiComposerFooterStaysInsideItsNarrowWidth() {
        shell.width = 520
        openAiWorkspace()
        const composer = shell.workspaceItem.composer
        const footer = findChild(composer, "aiComposerFooter")
        const status = findChild(composer, "aiComposerFooterStatus")
        const hint = findChild(composer, "aiComposerFooterHint")
        verify(!!footer && !!status && !!hint, "Footer objects exist")
        tryVerify(() => status.x + status.width <= footer.width + 0.5)
        if (hint.visible)
            verify(hint.x + hint.width <= footer.width + 0.5)
    }
    function test_aiComposerKeepsDraftUntilAuthoritativeAcceptance() {
        openAiWorkspace()
        const composer = shell.workspaceItem.composer
        composer.editor.text = "Finish the Qt composer migration"
        composer.submit()
        compare(fakeBackend.lastCommand, "steer_ai_prompt")
        compare(fakeBackend.lastArgument, "Finish the Qt composer migration")
        verify(fakeBackend.aiPromptPending)
        verify(composer.submitting)
        compare(composer.editor.text, "Finish the Qt composer migration")
        verify(!composer.editor.enabled)
        fakeBackend.accept_ai_prompt()
        verify(!composer.submitting)
        compare(composer.editor.text, "")
        verify(composer.editor.enabled)
    }
    function test_aiComposerSendsAnImageOnlyPromptAndClearsAfterAcceptance() {
        fakeBackend.aiTurnRunning = false
        fakeBackend.aiActiveTurnId = ""
        openAiWorkspace()
        const composer = shell.workspaceItem.composer
        fakeBackend.add_ai_attachments(JSON.stringify(["state.png"]))
        compare(fakeBackend.aiAttachmentCount, 1)
        tryVerify(() => composer.canSubmit)
        composer.submit()
        compare(fakeBackend.lastCommand, "send_ai_prompt")
        compare(fakeBackend.lastArgument, "")
        verify(fakeBackend.aiPromptPending)
        compare(fakeBackend.aiAttachmentCount, 1)
        fakeBackend.accept_ai_prompt()
        compare(fakeBackend.aiAttachmentCount, 0)
        verify(!composer.submitting)
    }
    function test_aiComposerRestoresRejectedDraftForEditing() {
        openAiWorkspace()
        const composer = shell.workspaceItem.composer
        composer.editor.text = "Keep this draft if sending fails"
        composer.submit()
        fakeBackend.fail_ai_prompt()
        verify(!composer.submitting)
        compare(composer.editor.text, "Keep this draft if sending fails")
        verify(composer.editor.enabled)
    }
    function test_aiComposerRestoresFocusAfterAttachmentValidation() {
        openAiWorkspace()
        const composer = shell.workspaceItem.composer
        fakeBackend.aiAttachmentPending = true
        fakeBackend.aiStateChanged()
        verify(!composer.editor.enabled)
        fakeBackend.aiAttachmentPending = false
        fakeBackend.aiStateChanged()
        tryVerify(() => composer.editor.activeFocus)
    }
    function test_aiComposerRoutesSendSteerAndKeyboardControls() {
        fakeBackend.aiTurnRunning = false
        fakeBackend.aiActiveTurnId = ""
        openAiWorkspace()
        const composer = shell.workspaceItem.composer
        compare(composer.sendButton.label, "Send")
        composer.editor.forceActiveFocus()
        composer.editor.text = "First line"
        composer.editor.cursorPosition = composer.editor.text.length
        keyClick(Qt.Key_Return, Qt.ShiftModifier)
        compare(composer.editor.text, "First line\n")
        composer.editor.text = "Start a new turn"
        keyClick(Qt.Key_Return)
        compare(fakeBackend.lastCommand, "send_ai_prompt")
        fakeBackend.accept_ai_prompt()
        fakeBackend.aiTurnRunning = true
        fakeBackend.aiActiveTurnId = "turn-next"
        fakeBackend.aiStateChanged()
        compare(composer.sendButton.label, "Steer")
        composer.editor.text = "Adjust the active turn"
        composer.submit()
        compare(fakeBackend.lastCommand, "steer_ai_prompt")
    }
    function test_aiComposerQueuesAndEditsFollowUpsWithKeyboardControls() {
        openAiWorkspace()
        const workspace = shell.workspaceItem
        const composer = workspace.composer
        composer.editor.forceActiveFocus()
        composer.editor.text = "Run the focused tests after this turn"
        keyClick(Qt.Key_Tab)
        compare(fakeBackend.lastCommand, "queue_ai_follow_up")
        compare(fakeBackend.lastArgument, "Run the focused tests after this turn")
        compare(composer.editor.text, "")
        compare(fakeBackend.aiActiveQueuedMessageCount, 1)
        tryCompare(workspace.timelineListView, "count", 5)
        composer.editor.text = "Then review the queue lifecycle"
        keyClick(Qt.Key_Tab)
        compare(fakeBackend.aiActiveQueuedMessageCount, 2)
        composer.editor.text = "Preserve this draft"
        const commandCount = fakeBackend.commandCount
        keyClick(Qt.Key_Up, Qt.ControlModifier | Qt.ShiftModifier)
        compare(fakeBackend.commandCount, commandCount)
        compare(composer.editor.text, "Preserve this draft")
        compare(fakeBackend.aiActiveQueuedMessageCount, 2)
        composer.editor.text = ""
        keyClick(Qt.Key_Up, Qt.ControlModifier | Qt.ShiftModifier)
        compare(fakeBackend.lastCommand, "edit_last_ai_queued_prompt")
        compare(composer.editor.text, "Then review the queue lifecycle")
        compare(fakeBackend.aiActiveQueuedMessageCount, 1)
    }
    function test_aiQueuedFollowUpWaitsForAcceptanceBeforeLeavingTimeline() {
        openAiWorkspace()
        const workspace = shell.workspaceItem
        const composer = workspace.composer
        composer.editor.text = "Continue after the current turn"
        composer.queueFollowUp()
        fakeBackend.finish_ai_turn()
        compare(fakeBackend.lastCommand, "send_queued_ai_prompt")
        verify(fakeBackend.aiActiveQueueSending)
        verify(!composer.editor.enabled)
        const queuedIndex = fakeBackend.first_queued_ai_message_index()
        compare(aiTimelineModel.get(queuedIndex).status, "sending")
        fakeBackend.accept_queued_ai_prompt()
        compare(fakeBackend.aiActiveQueuedMessageCount, 0)
        verify(!fakeBackend.aiActiveQueueSending)
        verify(composer.editor.enabled)
        tryVerify(() => composer.editor.activeFocus)
    }
    function test_aiInterruptRestoresQueuedFollowUpsToTheThreadDraft() {
        openAiWorkspace()
        const composer = shell.workspaceItem.composer
        composer.editor.text = "Do not lose this queued follow-up"
        composer.queueFollowUp()
        composer.interrupt()
        fakeBackend.complete_ai_interrupt()
        compare(fakeBackend.aiActiveQueuedMessageCount, 0)
        compare(composer.editor.text, "Do not lose this queued follow-up")
        verify(composer.editor.enabled)
        tryVerify(() => composer.editor.activeFocus)
    }
    function test_aiRecoveryPreservesDistinctSubstringDrafts() {
        openAiWorkspace()
        const composer = shell.workspaceItem.composer
        composer.editor.text = "don't run tests"
        fakeBackend.recoveredAiPrompts[fakeBackend.aiActiveThreadId] = "run tests"
        fakeBackend.aiStateChanged()
        compare(composer.editor.text, "don't run tests\n\nrun tests")
    }
    function test_aiRecoveryRestoresAnImageOnlyDraft() {
        openAiWorkspace()
        const composer = shell.workspaceItem.composer
        fakeBackend.recoveredAiAttachmentPaths[fakeBackend.aiActiveThreadId] = ["restored.png"]
        fakeBackend.aiStateChanged()
        tryCompare(aiAttachmentsModel, "count", 1)
        tryVerify(() => composer.canSubmit)
    }
    function test_aiIdleTabDoesNotSendTheDraft() {
        fakeBackend.aiTurnRunning = false
        fakeBackend.aiActiveTurnId = ""
        openAiWorkspace()
        const composer = shell.workspaceItem.composer
        composer.editor.forceActiveFocus()
        composer.editor.text = "Keep this idle draft"
        const commandCount = fakeBackend.commandCount
        keyClick(Qt.Key_Tab)
        compare(fakeBackend.commandCount, commandCount)
        verify(!fakeBackend.aiPromptPending)
    }
    function test_aiComposerDraftsSurviveThreadAndWorkspaceSwitches() {
        openAiWorkspace()
        shell.workspaceItem.composer.editor.text = "Migration thread draft"
        shell.sidebarItem.selectThread("thread-review")
        compare(shell.workspaceItem.composer.editor.text, "")
        shell.workspaceItem.composer.editor.text = "Review thread draft"
        shell.sidebarItem.selectThread("thread-qt-migration")
        compare(shell.workspaceItem.composer.editor.text, "Migration thread draft")
        openDiffWorkspace()
        openAiWorkspace()
        compare(shell.workspaceItem.composer.editor.text, "Migration thread draft")
    }
    function test_aiComposerDisablesUnavailableAndDuplicateActions() {
        openAiWorkspace()
        const composer = shell.workspaceItem.composer
        composer.editor.text = "Blocked prompt"
        fakeBackend.aiRequiresAuthentication = true
        verify(!composer.sendButton.enabled)
        fakeBackend.aiRequiresAuthentication = false
        fakeBackend.aiLoading = true
        verify(!composer.sendButton.enabled)
        fakeBackend.aiLoading = false
        fakeBackend.aiActiveThreadId = ""
        fakeBackend.aiStateChanged()
        verify(!composer.sendButton.enabled)
        fakeBackend.aiActiveThreadId = "thread-qt-migration"
        fakeBackend.aiStateChanged()
        composer.editor.text = "Only send once"
        composer.submit()
        verify(!composer.sendButton.enabled)
        const commandCount = fakeBackend.commandCount
        composer.submit()
        compare(fakeBackend.commandCount, commandCount)
    }
    function test_aiComposerInterruptsOnlyTheExactActiveTurnOnce() {
        openAiWorkspace()
        const composer = shell.workspaceItem.composer
        verify(fakeBackend.aiTurnRunning)
        compare(composer.stopButton.label, "Stop")
        composer.interrupt()
        compare(fakeBackend.lastCommand, "interrupt_ai_turn")
        compare(fakeBackend.lastArgument, "turn-2")
        verify(fakeBackend.aiInterruptPending)
        verify(!composer.stopButton.enabled)
        const commandCount = fakeBackend.commandCount
        composer.interrupt()
        compare(fakeBackend.commandCount, commandCount)
        fakeBackend.complete_ai_interrupt()
        verify(!fakeBackend.aiTurnRunning)
    }
    function test_aiApprovalUsesExactIdsAndRestoresComposerFocus() {
        openAiWorkspace()
        const workspace = shell.workspaceItem
        const panel = workspace.requestPanel
        const composer = workspace.composer
        composer.editor.forceActiveFocus()
        verify(composer.editor.activeFocus)
        fakeBackend.show_ai_approval("approval-accept")

        tryVerify(() => panel.loadedRequestId === "approval-accept"
            && panel.acceptButton.enabled)
        tryVerify(() => panel.acceptButton.activeFocus)
        verify(!composer.editor.enabled)
        shell.sidebarItem.threadListView.forceLayout()
        const threadRow = shell.sidebarItem.threadListView.itemAtIndex(0)
        verify(threadRow.attention)
        verify(!threadRow.archiveButton.enabled)
        const archiveCommandCount = fakeBackend.commandCount
        fakeBackend.archive_ai_thread("thread-qt-migration")
        compare(fakeBackend.commandCount, archiveCommandCount)
        verify(panel.resolveApproval(true))
        compare(fakeBackend.lastCommand, "accept_ai_approval")
        compare(fakeBackend.lastArgument, "approval-accept")
        verify(fakeBackend.aiRequestResolving)
        const commandCount = fakeBackend.commandCount
        verify(!panel.resolveApproval(true))
        compare(fakeBackend.commandCount, commandCount)

        fakeBackend.complete_ai_request()
        tryVerify(() => composer.editor.enabled && composer.editor.activeFocus)
        fakeBackend.show_ai_approval("approval-decline")
        verify(!fakeBackend.resolve_ai_approval("stale-approval", false))
        verify(panel.resolveApproval(false))
        compare(fakeBackend.lastCommand, "decline_ai_approval")
        compare(fakeBackend.lastArgument, "approval-decline")
    }
    function test_aiUserInputRetainsAnswersOnFailureAndMasksSecrets() {
        openAiWorkspace()
        let panel = shell.workspaceItem.requestPanel
        const questions = JSON.stringify([
            {
                id: "approach",
                header: "Approach",
                question: "Which approach should Codex use?",
                isOther: true,
                isSecret: false,
                options: [
                    { label: "Simple", description: "Keep it narrow." },
                    { label: "Broad", description: "Include adjacent work." }
                ]
            },
            {
                id: "token",
                header: "Token",
                question: "Provide the temporary token.",
                isOther: true,
                isSecret: true,
                options: []
            }
        ])
        fakeBackend.show_ai_user_input("input-1", questions)

        compare(panel.questions.length, 2)
        compare(panel.answerFor("approach"), "Simple")
        const answerInput = findChild(panel, "aiAnswerInput-approach")
        verify(answerInput !== null)
        answerInput.forceActiveFocus()
        answerInput.text = "Custom"
        answerInput.textEdited()
        compare(panel.answerFor("approach"), "Custom")
        panel.setAnswer("approach", "Broad")
        compare(answerInput.text, "Broad")
        panel.setAnswer("token", "do-not-log-this")
        const secretInput = findChild(panel, "aiSecretAnswerInput")
        verify(secretInput !== null)
        compare(secretInput.echoMode, TextInput.Password)

        fakeBackend.show_ai_approval("approval-other-thread")
        fakeBackend.show_ai_user_input("input-1", questions)
        compare(panel.answerFor("approach"), "Broad")
        compare(panel.answerFor("token"), "do-not-log-this")

        shell.activateWorkspace("diff")
        tryVerify(() => shell.workspaceItem.objectName === "diffWorkspace")
        shell.activateWorkspace("ai")
        tryVerify(() => shell.workspaceItem.objectName === "aiWorkspace")
        panel = shell.workspaceItem.requestPanel
        tryCompare(panel, "loadedRequestId", "input-1")
        compare(panel.answerFor("approach"), "Broad")
        compare(panel.answerFor("token"), "do-not-log-this")

        fakeBackend.failNextAiRequest = true
        verify(!panel.submitInput())
        compare(panel.answerFor("approach"), "Broad")
        compare(panel.answerFor("token"), "do-not-log-this")
        verify(panel.submitInput())
        compare(fakeBackend.lastCommand, "submit_ai_user_input")
        compare(fakeBackend.lastArgument, "input-1")
        const payload = JSON.parse(fakeBackend.lastAnswersJson)
        compare(payload.approach[0], "Broad")
        compare(payload.token[0], "do-not-log-this")
        verify(!panel.submitButton.enabled)

        fakeBackend.complete_ai_request()
        tryVerify(() => shell.aiRequestAnswerStore["input-1"] === undefined)
    }
    function test_aiRequestKeyboardFocusScrollsOverflow() {
        openAiWorkspace()
        const panel = shell.workspaceItem.requestPanel
        const questions = []
        for (let index = 0; index < 8; ++index) {
            questions.push({
                id: "question-" + index,
                header: "Question " + (index + 1),
                question: "Provide answer " + (index + 1) + ".",
                isOther: false,
                isSecret: false,
                options: []
            })
        }
        fakeBackend.show_ai_user_input("input-scroll", JSON.stringify(questions))

        tryCompare(panel, "loadedRequestId", "input-scroll")
        const lastInput = findChild(panel, "aiAnswerInput-question-7")
        verify(lastInput !== null)
        verify(lastInput.activeFocusOnTab)
        tryVerify(() => panel.requestViewport.contentHeight
            > panel.requestViewport.height)
        lastInput.forceActiveFocus()
        tryVerify(() => lastInput.activeFocus)
        tryVerify(() => panel.requestViewport.contentY > 0)
    }
    function test_aiOversizedInputCannotBeSubmitted() {
        openAiWorkspace()
        const panel = shell.workspaceItem.requestPanel
        fakeBackend.show_ai_user_input("too-large", "[]")
        fakeBackend.aiRequestAnswerable = false
        fakeBackend.aiStateChanged()

        tryVerify(() => panel.loadedRequestId === "too-large"
            && !panel.submitButton.enabled)
        verify(!panel.submitInput())
        compare(fakeBackend.lastCommand, "")
    }
    function test_diffWorkspaceUsesVirtualizedRustModels() {
        openDiffWorkspace()
        shell.workspaceItem.diffListView.forceLayout()
        compare(shell.workspaceItem.diffListView.count, 3)
        verify(shell.workspaceItem.diffListView.reuseItems)
        compare(shell.sidebarItem.fileListView.count, 2)
        verify(shell.sidebarItem.fileListView.reuseItems)
    }
    function test_diffSplitDividerBalancesAndKeepsBoundedPaneWidths() {
        openDiffWorkspace()
        tryCompare(shell.workspaceItem.commentsInspector, "width", 0)
        const viewport = shell.workspaceItem.diffViewport
        const divider = findChild(shell.workspaceItem, "diffSplitHandle")
        const splitPointer = findChild(shell.workspaceItem,
            "diffSplitPointer")
        verify(!!viewport, "Object exists")
        verify(!!divider, "Object exists")
        verify(!!splitPointer, "Object exists")
        compare(splitPointer.hoverEnabled, true)
        verify(divider.width > 0)
        verify(divider.height > 0)
        compare(Math.round(shell.workspaceItem.splitPosition),
            Math.round(viewport.width / 2))
        compare(divider.activeFocusOnTab, true)
        compare(divider.Accessible.focusable, true)

        shell.workspaceItem.diffListView.forceLayout()
        const changedRow = shell.workspaceItem.diffListView.itemAtIndex(1)
        verify(!!changedRow, "Object exists")
        const beforeCell = findChild(changedRow, "diffBeforeCell")
        const afterCell = findChild(changedRow, "diffAfterCell")
        const beforeContent = findChild(changedRow, "diffBeforeContent")
        const afterContent = findChild(changedRow, "diffAfterContent")
        verify(!!beforeCell, "Object exists")
        verify(!!afterCell, "Object exists")
        verify(!!beforeContent, "Object exists")
        verify(!!afterContent, "Object exists")
        compare(beforeCell.width, shell.workspaceItem.splitPosition)
        compare(afterCell.width,
            viewport.width - shell.workspaceItem.splitPosition)

        const initialPosition = shell.workspaceItem.splitPosition
        shell.workspaceItem.setSplitPosition(initialPosition + 80)
        compare(shell.workspaceItem.splitPosition, initialPosition + 80)
        compare(beforeCell.width, shell.workspaceItem.splitPosition)
        compare(afterCell.width,
            viewport.width - shell.workspaceItem.splitPosition)

        shell.workspaceItem.setSplitPosition(0)
        compare(shell.workspaceItem.splitPosition,
            shell.workspaceItem.splitMinimumPaneWidth)
        shell.workspaceItem.setSplitPosition(viewport.width)
        compare(shell.workspaceItem.splitPosition,
            viewport.width - shell.workspaceItem.splitMinimumPaneWidth)

        shell.workspaceItem.setSplitPosition(viewport.width / 2)
        const scrollOffset = Math.min(120,
            viewport.contentWidth - viewport.width)
        verify(scrollOffset > 0)
        viewport.contentX = scrollOffset
        compare(Math.round(beforeCell.mapToItem(viewport, 0, 0).x), 0)
        compare(Math.round(afterCell.mapToItem(viewport, 0, 0).x),
            Math.round(shell.workspaceItem.splitPosition))

        divider.forceActiveFocus()
        tryCompare(divider, "activeFocus", true)
        const keyboardPosition = shell.workspaceItem.splitPosition
        keyClick(Qt.Key_Right)
        compare(Math.round(shell.workspaceItem.splitPosition),
            Math.round(keyboardPosition + 16))

        shell.workspaceItem.setSplitPosition(
            shell.workspaceItem.splitMinimumPaneWidth)
        viewport.contentX = viewport.contentWidth - viewport.width
        compare(Math.round(beforeCell.mapToItem(viewport, 0, 0).x), 0)
        compare(Math.round(afterCell.mapToItem(viewport, 0, 0).x),
            Math.round(shell.workspaceItem.splitPosition))
        compare(Math.round(beforeContent.mapToItem(viewport,
            beforeContent.width, 0).x),
            Math.round(shell.workspaceItem.splitMinimumPaneWidth))
        compare(Math.round(afterContent.mapToItem(viewport,
            afterContent.width, 0).x),
            Math.round(shell.workspaceItem.splitPosition
                + shell.workspaceItem.splitMinimumPaneWidth))

        divider.forceActiveFocus()
        tryCompare(divider, "activeFocus", true)
        shell.workspaceItem.setDiffMode("unified")
        compare(divider.visible, false)
        compare(divider.activeFocus, false)
        const unifiedPosition = shell.workspaceItem.splitPosition
        keyClick(Qt.Key_Right)
        compare(shell.workspaceItem.splitPosition, unifiedPosition)
        shell.workspaceItem.setDiffMode("split")
        shell.workspaceItem.setSplitPosition(viewport.width / 2)
    }
    function test_diffFileSelectionUsesBackendCommand() {
        openDiffWorkspace()
        shell.sidebarItem.fileListView.forceLayout()
        const secondFile = shell.sidebarItem.fileListView.itemAtIndex(1)
        verify(secondFile !== null)
        shell.sidebarItem.activateFile("crates/hunk-desktop/src/qml/Hunk/DiffWorkspace.qml")
        compare(fakeBackend.lastCommand, "select_diff_file")
        compare(fakeBackend.lastArgument, "crates/hunk-desktop/src/qml/Hunk/DiffWorkspace.qml")
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
        tryVerify(() => shell.workspaceItem.activeFocus)

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
                file_path: "crates/hunk-desktop/src/backend.rs",
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
        captureSnapshot("target/hunk-desktop-diff-comments.png")
    }
    function test_diffWorkspaceRendersAtDesktopSize() {
        openDiffWorkspace()
        shell.workspaceItem.searchInput.text = ""
        shell.workspaceItem.applySearch("")
        shell.workspaceItem.selectRow(2, false)
        captureSnapshot("target/hunk-desktop-diff.png")
        shell.workspaceItem.resetSelection()
    }
    function test_workspaceActivationUsesBackendCommand() {
        openAiWorkspace()
        compare(fakeBackend.lastRequestedWorkspace, "ai")
        compare(shell.activeWorkspace, "ai")
    }
    function test_terminalRestoresAiComposerFocus() {
        openAiWorkspace()
        verifyTerminalFocusRoundTrip(shell.workspaceItem.composer.editor)
    }
    // macOS's offscreen native dialog backend does not reactivate the test
    // window after close, so keep this integration check last in the suite.
    function test_z_aiRepositoryPickerOpensAndRestoresFocus() {
        openAiWorkspace()
        const editor = shell.workspaceItem.composer.editor
        editor.forceActiveFocus()
        tryVerify(() => editor.activeFocus)
        shell.sidebarItem.openRepository()
        tryCompare(shell.sidebarItem.repositoryDialog, "visible", true)
        shell.sidebarItem.repositoryDialog.close()
        tryCompare(shell.sidebarItem.repositoryDialog, "visible", false)
        tryVerify(() => editor.activeFocus)
    }
}
