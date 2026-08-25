pragma ComponentBehavior: Bound

import QtQuick
import QtTest
import Hunk

Item {
    id: root

    width: 1280
    height: 760

    ListModel {
        id: aiThreadsModel
    }
    ListModel {
        id: aiTimelineModel
    }
    ListModel {
        id: aiAttachmentsModel
    }
    ListModel {
        id: aiModelsModel
    }
    ListModel {
        id: aiEffortsModel
    }
    ListModel {
        id: aiServiceTiersModel
    }
    ListModel {
        id: terminalTabsModel
    }
    ListModel {
        id: terminalRowsModel
    }
    ListModel {
        id: browserTabs

        ListElement {
            tab_id: "tab-1"
            title: "New tab"
            url: "about:blank"
            loading: false
        }
    }

    FakeBrowser {
        id: fakeBrowser

        tabs: browserTabs
    }

    QtObject {
        id: fakeBackend

        property string activeWorkspace: "ai"
        property bool ready: true
        property string statusMessage: "Test backend ready"
        property QtObject browser: fakeBrowser
        property string gitRepositoryName: "hunk"
        property bool terminalOpen: false
        property int terminalFocusRevision: 0
        property QtObject terminalTabs: terminalTabsModel
        property QtObject terminalRows: terminalRowsModel
        property int terminalActiveTabId: 1
        property int terminalActiveTabIndex: 0
        property string terminalShellLabel: "zsh"
        property string terminalStatus: "idle"
        property string terminalStatusMessage: ""
        property string terminalCwd: "/repo"
        property int terminalDisplayOffset: 0
        property bool terminalMouseMode: false
        property int terminalCursorRow: -1
        property int terminalCursorColumn: -1
        property string terminalCursorShape: "hidden"
        property bool terminalCursorVisible: false
        property int terminalScreenRevision: 0
        property QtObject aiThreads: aiThreadsModel
        property QtObject aiTimeline: aiTimelineModel
        property QtObject aiAttachments: aiAttachmentsModel
        property QtObject aiModels: aiModelsModel
        property QtObject aiEfforts: aiEffortsModel
        property QtObject aiServiceTiers: aiServiceTiersModel
        property bool aiReady: true
        property bool aiLoading: false
        property bool aiRequiresAuthentication: false
        property string aiConnectionState: "ready"
        property string aiWorkspaceRoot: "/repo"
        property string aiActiveThreadId: "thread-qt-migration"
        property string aiActiveThreadTitle: "Replace GPUI with Qt"
        property string aiActiveThreadCwd: "/repo"
        property bool aiTurnRunning: false
        property bool aiThreadActionPending: false
        property bool aiPromptPending: false
        property bool aiAttachmentPending: false
        property bool aiModelSupportsImageInputs: true
        property bool aiInterruptPending: false
        property bool aiActiveQueueSending: false
        property int aiActiveQueuedMessageCount: 0
        property int aiPromptAcceptedRevision: 0
        property int aiPendingRequestCount: 0
        property string aiRequestId: ""
        property string aiRequestKind: ""
        property string aiRequestTitle: ""
        property string aiRequestDescription: ""
        property string aiRequestReason: ""
        property string aiRequestQuestionsJson: "[]"
        property bool aiRequestAnswerable: false
        property bool aiRequestResolving: false
        property int aiThreadCount: 1
        property int aiRunningThreadCount: 0
        property int aiTimelineTotalTurnCount: 0
        property int aiTimelineVisibleTurnCount: 0
        property int aiTimelineHiddenTurnCount: 0
        property int aiTimelineHiddenRowCount: 0
        property string aiError: ""
        property string aiStatusMessage: ""
        property int aiSelectedModelIndex: -1
        property int aiSelectedEffortIndex: -1
        property int aiSelectedServiceTierIndex: -1
        property int aiEffortOptionCount: 0
        property string aiSelectedModelLabel: "Default"
        property string aiSelectedEffortLabel: "Default"
        property string aiSelectedCollaborationMode: "code"
        property bool aiMadMaxMode: false
        property bool aiSessionControlsLocked: false
        property bool aiContextAvailable: false
        property int aiContextPercentUsed: 0
        property int aiContextPercentLeft: 100
        property string aiContextTokenSummary: ""
        property string aiContextInputTokens: "0"
        property string aiContextCachedInputTokens: "0"
        property string aiContextOutputTokens: "0"
        property string aiContextReasoningTokens: "0"
        property string aiContextBillableTokens: "0"

        signal aiStateChanged
        signal terminalStateChanged
        signal terminalScreenChanged
        signal terminalFocusChanged

        function select_workspace(workspace) {
            activeWorkspace = workspace;
        }
        function ai_request_pending() {
            return false;
        }
        function take_ai_recovered_prompt() {
            return "";
        }
        function refresh_ai_threads() {
            return true;
        }
        function create_ai_thread() {
            return true;
        }
        function select_ai_thread() {
            return true;
        }
        function toggle_ai_thread_bookmark() {
            return true;
        }
        function archive_ai_thread() {
            return true;
        }
        function fork_ai_thread() {
            return true;
        }
        function add_ai_attachments() {
            return true;
        }
        function remove_ai_attachment() {
            return true;
        }
        function resolve_ai_approval() {
            return true;
        }
        function submit_ai_user_input() {
            return true;
        }
        function send_ai_prompt() {
            return true;
        }
        function queue_ai_follow_up() {
            return true;
        }
        function edit_last_ai_queued_prompt() {
            return true;
        }
        function interrupt_ai_turn() {
            return true;
        }
        function select_ai_collaboration_mode() {
            return true;
        }
        function select_ai_model() {
            return true;
        }
        function select_ai_effort() {
            return true;
        }
        function select_ai_service_tier() {
            return true;
        }
        function set_ai_mad_max_mode() {
            return true;
        }
        function set_terminal_open(value) {
            terminalOpen = value;
            terminalStateChanged();
            return true;
        }
        function new_terminal_tab() {
            return true;
        }
        function select_terminal_tab() {
            return true;
        }
        function close_terminal_tab() {
            return true;
        }
        function move_terminal_tab() {
            return true;
        }
        function clear_terminal_screen() {
            return true;
        }
        function report_terminal_focus() {
            return true;
        }
        function resize_terminal() {
            return true;
        }
        function terminal_selection_text() {
            return "";
        }
        function terminal_pointer_button() {
            return true;
        }
        function terminal_pointer_move() {
            return true;
        }
        function terminal_wheel() {
            return true;
        }
        function send_terminal_key() {
            return true;
        }
        function scroll_terminal() {
            return true;
        }
        function paste_terminal_text() {
            return true;
        }
        function write_terminal_text() {
            return true;
        }
        function run_terminal_command() {
            return true;
        }
    }

    Component {
        id: fakeBrowserSurface

        Rectangle {
            property bool hasFrame: true
            color: Theme.input
        }
    }

    Shell {
        id: shell

        width: root.width
        height: root.height
        backend: fakeBackend
        browserSurfaceComponent: fakeBrowserSurface
    }

    TestCase {
        name: "BrowserShellTests"
        when: windowShown

        function init() {
            root.height = 760;
            fakeBackend.activeWorkspace = "ai";
            fakeBackend.aiActiveThreadId = "thread-qt-migration";
            fakeBackend.terminalOpen = false;
            fakeBrowser.activeThreadId = "thread-qt-migration";
            fakeBrowser.open = false;
            fakeBrowser.allowOpen = true;
            fakeBrowser.approvalPending = false;
            fakeBrowser.stateChanged();
            fakeBackend.aiStateChanged();
            wait(0);
            tryVerify(() => shell.workspaceItem !== null && shell.workspaceItem.objectName === "aiWorkspace");
        }

        function test_aiWorkspaceTogglesTheRetainedBrowserPane() {
            const browserButton = findChild(shell.workspaceItem, "browserAction");
            verify(!!browserButton, "Object exists");
            verify(!!findChild(shell.workspaceItem, "browserPane"), "Object exists");
            verify(browserButton.enabled);
            browserButton.clicked();
            tryCompare(fakeBrowser, "open", true);
            tryCompare(shell.workspaceItem, "browserVisible", true);
            tryCompare(browserButton, "label", qsTr("Conversation"));
            browserButton.clicked();
            tryCompare(fakeBrowser, "open", false);
            tryCompare(shell.workspaceItem, "browserVisible", false);
            tryCompare(browserButton, "label", qsTr("Browser"));
        }

        function test_openBrowserCanCloseAfterTheActiveThreadClears() {
            const browserButton = findChild(shell.workspaceItem, "browserAction");
            browserButton.clicked();
            tryCompare(fakeBrowser, "open", true);
            fakeBackend.aiActiveThreadId = "";
            fakeBrowser.activeThreadId = "";
            fakeBackend.aiStateChanged();
            fakeBrowser.stateChanged();
            verify(browserButton.enabled);
            browserButton.clicked();
            tryCompare(fakeBrowser, "open", false);
        }

        function test_rejectedBrowserOpenDoesNotRetainStaleFocusState() {
            const browserButton = findChild(shell.workspaceItem, "browserAction");
            fakeBrowser.allowOpen = false;
            browserButton.clicked();
            tryCompare(fakeBrowser, "open", false);
            tryCompare(shell.workspaceItem, "browserPreviousFocusItem", null);
        }

        function test_browserApprovalRestoresTheDisplacedFocusOwner() {
            const browserButton = findChild(shell.workspaceItem, "browserAction");
            browserButton.forceActiveFocus();
            tryCompare(browserButton, "activeFocus", true);
            fakeBrowser.approvalPending = true;
            fakeBrowser.approvalChanged();
            tryCompare(browserButton, "activeFocus", false);
            fakeBrowser.approvalPending = false;
            fakeBrowser.approvalChanged();
            tryCompare(browserButton, "activeFocus", true);
        }

        function test_browserApprovalFallsBackToTheReadyBrowserSurface() {
            const editor = shell.workspaceItem.composer.editor;
            const browserInput = findChild(shell.workspaceItem, "browserInput");
            editor.forceActiveFocus();
            tryCompare(editor, "activeFocus", true);

            fakeBrowser.approvalPending = true;
            fakeBrowser.approvalChanged();
            fakeBrowser.open = true;
            fakeBrowser.stateChanged();
            tryCompare(shell.workspaceItem, "browserVisible", true);
            tryCompare(editor, "visible", false);
            fakeBrowser.approvalPending = false;
            fakeBrowser.approvalChanged();
            tryCompare(browserInput, "activeFocus", true);
        }

        function test_browserKeepsAUsableSurfaceAboveTheTerminalAtMinimumHeight() {
            root.height = 600;
            const browserButton = findChild(shell.workspaceItem, "browserAction");
            browserButton.clicked();
            shell.setTerminalOpen(true);
            const browserSurface = findChild(shell.workspaceItem, "browserSurface");
            verify(!!browserSurface, "Object exists");
            verify(browserSurface.height > 0);
        }
    }
}
