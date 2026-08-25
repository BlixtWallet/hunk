import QtQuick
import QtTest

import "../../src/qml/Hunk"

Item {
    id: root
    width: 900
    height: 420

    ListModel {
        id: terminalTabsModel

        ListElement { tab_id: 1; title: "zsh"; status: "running" }
    }

    ListModel {
        id: terminalRowsModel

        ListElement { row: 0; text: "ready"; markup: "ready" }
    }

    QtObject {
        id: fakeBackend

        property var terminalTabs: terminalTabsModel
        property var terminalRows: terminalRowsModel
        property int terminalActiveTabId: 1
        property int terminalActiveTabIndex: 0
        property string terminalShellLabel: "zsh"
        property string terminalStatus: "running"
        property string terminalStatusMessage: ""
        property string terminalCwd: "/repo"
        property int terminalDisplayOffset: 0
        property bool terminalMouseMode: false
        property int terminalCursorRow: 0
        property int terminalCursorColumn: 0
        property string terminalCursorShape: "block"
        property bool terminalCursorVisible: true
        property int terminalScreenRevision: 0
        property int terminalFocusRevision: 0
        signal terminalStateChanged
        signal terminalScreenChanged
        signal terminalFocusChanged

        function new_terminal_tab() { return true }
        function close_terminal_tab() { return true }
        function select_terminal_tab() { return true }
        function move_terminal_tab() { return true }
        function terminal_selection_text() { return qsTr("") }
        function report_terminal_focus() { return true }
        function write_terminal_text() { return true }
        function paste_terminal_text() { return true }
        function send_terminal_key() { return true }
        function scroll_terminal() { return true }
        function terminal_pointer_button() { return true }
        function terminal_pointer_move() { return true }
        function terminal_wheel() { return true }
        function clear_terminal_screen() { return true }
        function resize_terminal() { return true }
    }

    Component {
        id: terminalDrawerComponent

        TerminalDrawer {
            width: 860
            height: 360
            backend: fakeBackend
        }
    }

    TestCase {
        name: "TerminalDrawerTests"
        when: windowShown

        function init() {
            terminalTabsModel.clear()
            terminalTabsModel.append({ tab_id: 1, title: "zsh", status: "running" })
            fakeBackend.terminalActiveTabId = 1
            fakeBackend.terminalActiveTabIndex = 0
        }

        function test_tabsUseTheSharedBackendModel() {
            const drawer = createTemporaryObject(terminalDrawerComponent, root)
            verify(!!drawer, "Component exists")
            const tabs = findChild(drawer, "terminalTabs")
            verify(!!tabs, "Object exists")

            tryCompare(tabs, "count", 1)
            compare(drawer.screen.backend, fakeBackend)
        }

        function test_focusTerminalMovesFocusToTheScreenInput() {
            const drawer = createTemporaryObject(terminalDrawerComponent, root)
            verify(!!drawer, "Component exists")

            drawer.focusTerminal()

            tryCompare(drawer.screen.inputItem, "activeFocus", true)
        }

        function test_activeOverflowTabIsBroughtIntoView() {
            for (let index = 2; index <= 12; ++index) {
                terminalTabsModel.append({
                    tab_id: index,
                    title: "long-shell-tab-" + index,
                    status: "running"
                })
            }
            fakeBackend.terminalActiveTabId = 12
            fakeBackend.terminalActiveTabIndex = 11
            const drawer = createTemporaryObject(terminalDrawerComponent, root)
            verify(!!drawer, "Component exists")
            const tabs = findChild(drawer, "terminalTabs")
            verify(!!tabs, "Object exists")
            tabs.forceLayout()
            tryVerify(() => tabs.itemAtIndex(11) !== null)
            const activeTab = tabs.itemAtIndex(11)

            verify(activeTab.x >= tabs.contentX)
            verify(activeTab.x + activeTab.width <= tabs.contentX + tabs.width + 1)
        }
    }
}
