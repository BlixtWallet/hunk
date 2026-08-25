import QtQuick
import QtTest

import "../../src/qml/Hunk"

Item {
    id: root
    width: 720
    height: 320

    ListModel {
        id: terminalRowsModel
    }

    QtObject {
        id: fakeBackend

        property var terminalRows: terminalRowsModel
        property int terminalCursorRow: -1
        property int terminalCursorColumn: -1
        property string terminalCursorShape: "hidden"
        property bool terminalCursorVisible: false
        property int terminalDisplayOffset: 0
        property bool terminalMouseMode: false
        property int terminalScreenRevision: 0
        property int terminalFocusRevision: 0
        property string lastText: ""
        property string lastPaste: ""
        property int resizeCalls: 0
        property int lastRows: 0
        property int lastColumns: 0
        signal terminalStateChanged
        signal terminalScreenChanged
        signal terminalFocusChanged

        function terminal_selection_text() { return qsTr("selected text") }
        function report_terminal_focus() { return true }
        function write_terminal_text(text) { lastText = text; return true }
        function paste_terminal_text(text) { lastPaste = text; return true }
        function send_terminal_key() { return true }
        function scroll_terminal() { return true }
        function terminal_pointer_button() { return true }
        function terminal_pointer_move() { return true }
        function terminal_wheel() { return true }
        function clear_terminal_screen() { return true }
        function resize_terminal(rows, columns) {
            resizeCalls += 1
            lastRows = rows
            lastColumns = columns
            return true
        }
    }

    Component {
        id: terminalScreenComponent

        TerminalScreen {
            width: 640
            height: 240
            backend: fakeBackend
        }
    }

    TestCase {
        name: "TerminalScreenTests"
        when: windowShown

        function init() {
            terminalRowsModel.clear()
            fakeBackend.lastText = ""
            fakeBackend.lastPaste = ""
            fakeBackend.resizeCalls = 0
            fakeBackend.lastRows = 0
            fakeBackend.lastColumns = 0
        }

        function test_gridPointClampsToVisibleRows() {
            terminalRowsModel.append({ row: 0, text: qsTr("one"), markup: qsTr("one") })
            terminalRowsModel.append({ row: 1, text: qsTr("two"), markup: qsTr("two") })
            const screen = createTemporaryObject(terminalScreenComponent, root)
            verify(!!screen, "Component exists")

            const before = screen.gridPoint(-10, -10)
            const after = screen.gridPoint(200, 1000)

            compare(before.row, 0)
            compare(before.column, 0)
            compare(after.row, 1)
            verify(after.column > 0)
        }

        function test_delegateReceivesTerminalRowRoles() {
            terminalRowsModel.append({
                row: 0,
                text: qsTr("red text"),
                markup: qsTr("@terminalRed@ red text")
            })
            const screen = createTemporaryObject(terminalScreenComponent, root)
            verify(!!screen, "Component exists")
            screen.rowList.forceLayout()
            tryVerify(() => screen.rowList.itemAtIndex(0) !== null)
            const row = screen.rowList.itemAtIndex(0)

            compare(row.row, 0)
            compare(row.lineMarkup, qsTr("@terminalRed@ red text"))
            verify(row.renderedMarkup.indexOf("@terminalRed@") < 0)
        }

        function test_inputAcceptsCharacters() {
            const screen = createTemporaryObject(terminalScreenComponent, root)
            verify(!!screen, "Component exists")
            const input = findChild(screen, "terminalInput")
            verify(!!input, "Object exists")
            input.focus = true

            input.text = qsTr("hello")

            compare(fakeBackend.lastText, qsTr("hello"))
            compare(input.text, qsTr(""))
        }

        function test_inputAcceptsNumbers() {
            const screen = createTemporaryObject(terminalScreenComponent, root)
            verify(!!screen, "Component exists")
            const input = findChild(screen, "terminalInput")
            verify(!!input, "Object exists")
            input.focus = true

            input.text = qsTr("12345")

            compare(fakeBackend.lastText, qsTr("12345"))
            compare(input.text, qsTr(""))
        }

        function test_inputAcceptsSpecialCharacters() {
            const screen = createTemporaryObject(terminalScreenComponent, root)
            verify(!!screen, "Component exists")
            const input = findChild(screen, "terminalInput")
            verify(!!input, "Object exists")
            input.focus = true

            input.text = qsTr("$HOME && echo '@'")

            compare(fakeBackend.lastText, qsTr("$HOME && echo '@'"))
            compare(input.text, qsTr(""))
        }

        function test_selectAllSpansTheVisibleModel() {
            terminalRowsModel.append({ row: 0, text: qsTr("one"), markup: qsTr("one") })
            terminalRowsModel.append({ row: 1, text: qsTr("two"), markup: qsTr("two") })
            const screen = createTemporaryObject(terminalScreenComponent, root)
            verify(!!screen, "Component exists")

            screen.selectAll()

            compare(screen.selectionAnchorRow, 0)
            compare(screen.selectionAnchorColumn, 0)
            compare(screen.selectionHeadRow, 1)
            verify(screen.selectionHeadColumn > 0)
        }

        function test_initialLayoutReportsGridDimensions() {
            const screen = createTemporaryObject(terminalScreenComponent, root)
            verify(!!screen, "Component exists")

            tryVerify(() => fakeBackend.resizeCalls > 0)
            verify(fakeBackend.lastRows > 0)
            verify(fakeBackend.lastColumns > 0)
        }

        function test_hiddenScreenPreservesTheLastGridSize() {
            const screen = createTemporaryObject(terminalScreenComponent, root)
            verify(!!screen, "Component exists")
            tryVerify(() => fakeBackend.resizeCalls > 0)
            const callsBeforeHide = fakeBackend.resizeCalls

            screen.visible = false
            screen.height = 0
            wait(80)

            compare(fakeBackend.resizeCalls, callsBeforeHide)
        }
    }
}
