import QtQuick

QtObject {
    id: root

    required property QtObject tabs
    property string activeThreadId: "thread-qt-migration"
    property string activeTabId: "tab-1"
    property int activeTabIndex: 0
    property string url: "about:blank"
    property string title: ""
    property string runtimeStatus: "ready"
    property string statusMessage: ""
    property bool loading: false
    property bool canGoBack: false
    property bool canGoForward: false
    property bool open: false
    property bool allowOpen: true
    property bool pumpActive: false
    property bool approvalPending: false
    property string approvalKind: ""
    property string approvalSummary: ""
    property string contextTargetJson: ""
    property string contextClipboardText: ""
    property string lastKeyPress: ""

    signal stateChanged
    signal approvalChanged
    signal contextChanged
    signal contextMenuRequested(int x, int y)

    function set_open(value) {
        if (value && !root.allowOpen)
            return false;
        root.open = value;
        root.stateChanged();
        return true;
    }
    function pump() {
        return false;
    }
    function resolve_approval(accept) {
        root.approvalPending = false;
        root.approvalChanged();
        return accept;
    }
    function navigate(value) {
        root.url = value;
        root.stateChanged();
        return true;
    }
    function go_back() {
        return false;
    }
    function go_forward() {
        return false;
    }
    function reload() {
        return true;
    }
    function stop() {
        return true;
    }
    function new_tab() {
        return true;
    }
    function select_tab() {
        return true;
    }
    function close_tab() {
        return true;
    }
    function toggle_devtools() {
        return true;
    }
    function resize() {
        return true;
    }
    function republish_frame() {
        return true;
    }
    function report_focus() {
        return true;
    }
    function mouse_move() {
        return true;
    }
    function mouse_click() {
        return true;
    }
    function wheel() {
        return true;
    }
    function key_press(keys) {
        root.lastKeyPress = keys;
        return true;
    }
    function text_input() {
        return true;
    }
    function context_action() {
        return true;
    }
}
