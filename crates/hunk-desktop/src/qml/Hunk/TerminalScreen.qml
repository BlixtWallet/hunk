pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls

Item {
    id: root

    required property QtObject backend
    readonly property alias inputItem: terminalInput
    readonly property alias rowList: rows
    readonly property real cellWidth: Math.max(1, terminalMetrics.advanceWidth("M"))
    readonly property real cellHeight: Math.ceil(terminalMetrics.height * 1.15)
    property int selectionAnchorRow: -1
    property int selectionAnchorColumn: -1
    property int selectionHeadRow: -1
    property int selectionHeadColumn: -1
    property bool selecting: false
    property bool cursorBlinkVisible: true
    property bool clearingInput: false
    property bool pasteCapture: false
    property bool selectionInvalidated: false
    property int selectionScreenRevision: -1
    property int reportedMouseButton: Qt.NoButton
    property int reportedMouseRow: 0
    property int reportedMouseColumn: 0
    property bool reportedMouseShift: false
    property bool reportedMouseControl: false
    property bool reportedMouseAlt: false
    property real wheelRemainder: 0
    readonly property int observedScreenRevision: backend.terminalScreenRevision

    function focusTerminal() {
        terminalInput.forceActiveFocus()
    }

    function resetSelection() {
        selectionAnchorRow = -1
        selectionAnchorColumn = -1
        selectionHeadRow = -1
        selectionHeadColumn = -1
        selecting = false
        selectionInvalidated = false
        selectionScreenRevision = -1
    }

    function gridPoint(x, y) {
        return {
            row: Math.max(0, Math.min(rows.count - 1,
                Math.floor(y / cellHeight))),
            column: Math.max(0, Math.floor(x / cellWidth))
        }
    }

    function eventModifiers(modifiers) {
        return {
            shift: (modifiers & Qt.ShiftModifier) !== 0,
            control: (modifiers & Qt.ControlModifier) !== 0,
            alt: (modifiers & Qt.AltModifier) !== 0,
            platform: (modifiers & Qt.MetaModifier) !== 0
        }
    }

    function terminalKeyName(event) {
        const names = {}
        names[Qt.Key_Return] = "enter"
        names[Qt.Key_Enter] = "enter"
        names[Qt.Key_Tab] = "tab"
        names[Qt.Key_Backtab] = "tab"
        names[Qt.Key_Backspace] = "backspace"
        names[Qt.Key_Escape] = "escape"
        names[Qt.Key_Up] = "up"
        names[Qt.Key_Down] = "down"
        names[Qt.Key_Left] = "left"
        names[Qt.Key_Right] = "right"
        names[Qt.Key_Home] = "home"
        names[Qt.Key_End] = "end"
        names[Qt.Key_PageUp] = "pageup"
        names[Qt.Key_PageDown] = "pagedown"
        names[Qt.Key_Delete] = "delete"
        names[Qt.Key_Insert] = "insert"
        for (let index = 1; index <= 12; ++index)
            names[Qt.Key_F1 + index - 1] = "f" + index
        if (names[event.key] !== undefined)
            return names[event.key]
        if (event.key >= Qt.Key_A && event.key <= Qt.Key_Z)
            return String.fromCharCode(event.key).toLowerCase()
        if (event.key >= Qt.Key_0 && event.key <= Qt.Key_9)
            return String.fromCharCode(event.key)
        const punctuation = {
            "45": "-", "61": "=", "91": "[", "93": "]",
            "92": "\\", "59": ";", "39": "'", "44": ",",
            "46": ".", "47": "/", "96": "`", "32": "space"
        }
        return punctuation[String(event.key)] || ""
    }

    function copySelection() {
        const selected = backend.terminal_selection_text(
            selectionAnchorRow, selectionAnchorColumn,
            selectionHeadRow, selectionHeadColumn)
        if (selected.length === 0)
            return false
        clipboardProxy.text = selected
        clipboardProxy.forceActiveFocus()
        clipboardProxy.selectAll()
        clipboardProxy.copy()
        clipboardProxy.deselect()
        focusTerminal()
        return true
    }

    function selectAll() {
        if (rows.count <= 0)
            return
        selectionAnchorRow = 0
        selectionAnchorColumn = 0
        selectionHeadRow = rows.count - 1
        selectionHeadColumn = Math.max(0, Math.floor(width / cellWidth) - 1)
    }

    function pasteFromClipboard() {
        pasteCapture = true
        terminalInput.paste()
        if (terminalInput.text.length === 0)
            pasteCapture = false
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.terminalBackground
    }

    FontMetrics {
        id: terminalMetrics
        font.family: Theme.monoFont
        font.pixelSize: 12
    }

    ListView {
        id: rows
        objectName: "terminalRows"
        anchors.fill: parent
        interactive: false
        clip: true
        model: root.backend.terminalRows
        reuseItems: true
        cacheBuffer: Math.max(0, height)

        delegate: TerminalRow {
            required property string markup

            width: rows.width
            lineMarkup: markup
            cellWidth: root.cellWidth
            cellHeight: root.cellHeight
            selectionAnchorRow: root.selectionAnchorRow
            selectionAnchorColumn: root.selectionAnchorColumn
            selectionHeadRow: root.selectionHeadRow
            selectionHeadColumn: root.selectionHeadColumn
        }
    }

    Rectangle {
        visible: root.backend.terminalCursorVisible
            && root.backend.terminalDisplayOffset === 0
            && terminalInput.activeFocus
            && root.cursorBlinkVisible
            && root.backend.terminalCursorShape !== "hidden"
        x: root.backend.terminalCursorColumn * root.cellWidth
        y: root.backend.terminalCursorRow * root.cellHeight
            + (root.backend.terminalCursorShape === "underline" ? root.cellHeight - 2 : 0)
        width: root.backend.terminalCursorShape === "beam" ? 2 : root.cellWidth
        height: root.backend.terminalCursorShape === "underline" ? 2 : root.cellHeight
        color: root.backend.terminalCursorShape === "hollow"
            ? Theme.transparent : Theme.terminalCursorMuted
        border.width: root.backend.terminalCursorShape === "hollow" ? 1 : 0
        border.color: Theme.terminalCursor
    }

    MouseArea {
        id: pointer
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        hoverEnabled: root.backend.terminalMouseMode
        cursorShape: Qt.IBeamCursor

        onPressed: mouse => {
            root.focusTerminal()
            const point = root.gridPoint(mouse.x, mouse.y)
            const modifiers = root.eventModifiers(mouse.modifiers)
            const reportsMouse = root.backend.terminalMouseMode && !modifiers.shift
            if (reportsMouse) {
                root.reportedMouseButton = mouse.button
                root.reportedMouseRow = point.row
                root.reportedMouseColumn = point.column
                root.reportedMouseShift = modifiers.shift
                root.reportedMouseControl = modifiers.control
                root.reportedMouseAlt = modifiers.alt
                root.backend.terminal_pointer_button(
                    point.row, point.column, mouse.button, true,
                    modifiers.shift, modifiers.control, modifiers.alt)
            } else if (mouse.button === Qt.RightButton) {
                contextMenu.popup()
            } else if (mouse.button === Qt.LeftButton) {
                root.selectionAnchorRow = point.row
                root.selectionAnchorColumn = point.column
                root.selectionHeadRow = point.row
                root.selectionHeadColumn = point.column
                root.selecting = true
                root.selectionInvalidated = false
                root.selectionScreenRevision = root.observedScreenRevision
            }
        }
        onPositionChanged: mouse => {
            const point = root.gridPoint(mouse.x, mouse.y)
            const modifiers = root.eventModifiers(mouse.modifiers)
            const reportsMouse = root.backend.terminalMouseMode && !modifiers.shift
            if (reportsMouse) {
                root.backend.terminal_pointer_move(
                    point.row, point.column, mouse.buttons,
                    modifiers.shift, modifiers.control, modifiers.alt)
            } else if (root.selecting && (mouse.buttons & Qt.LeftButton)) {
                root.selectionHeadRow = point.row
                root.selectionHeadColumn = point.column
            }
        }
        onReleased: mouse => {
            const point = root.gridPoint(mouse.x, mouse.y)
            if (root.reportedMouseButton !== Qt.NoButton) {
                root.backend.terminal_pointer_button(
                    point.row, point.column, root.reportedMouseButton, false,
                    root.reportedMouseShift, root.reportedMouseControl,
                    root.reportedMouseAlt)
            }
            root.reportedMouseButton = Qt.NoButton
            root.selecting = false
            if (root.selectionInvalidated
                    || root.selectionScreenRevision !== root.observedScreenRevision)
                root.resetSelection()
        }
        onCanceled: {
            if (root.reportedMouseButton !== Qt.NoButton) {
                root.backend.terminal_pointer_button(
                    root.reportedMouseRow, root.reportedMouseColumn,
                    root.reportedMouseButton, false, root.reportedMouseShift,
                    root.reportedMouseControl, root.reportedMouseAlt)
            }
            root.reportedMouseButton = Qt.NoButton
            root.resetSelection()
        }
        onWheel: wheel => {
            const point = root.gridPoint(wheel.x, wheel.y)
            const modifiers = root.eventModifiers(wheel.modifiers)
            const delta = wheel.pixelDelta.y !== 0
                ? wheel.pixelDelta.y / root.cellHeight
                : wheel.angleDelta.y / 40
            root.wheelRemainder += delta
            const lines = Math.trunc(root.wheelRemainder)
            if (lines === 0) {
                wheel.accepted = true
                return
            }
            root.wheelRemainder -= lines
            wheel.accepted = root.backend.terminal_wheel(
                point.row, point.column, lines,
                modifiers.shift, modifiers.control, modifiers.alt)
        }
    }

    TextInput {
        id: terminalInput
        objectName: "terminalInput"
        x: -2
        y: -2
        width: 1
        height: 1
        opacity: 0
        activeFocusOnTab: true
        Accessible.role: Accessible.EditableText
        Accessible.name: qsTr("Terminal input")

        onActiveFocusChanged: {
            root.backend.report_terminal_focus(activeFocus)
            if (activeFocus) {
                root.cursorBlinkVisible = true
                cursorTimer.restart()
            }
        }
        onTextChanged: {
            if (root.clearingInput || text.length === 0)
                return
            if (root.pasteCapture)
                root.backend.paste_terminal_text(text)
            else
                root.backend.write_terminal_text(text)
            root.clearingInput = true
            text = ""
            root.clearingInput = false
            root.pasteCapture = false
        }

        Keys.priority: Keys.BeforeItem
        Keys.onPressed: event => {
            const modifiers = root.eventModifiers(event.modifiers)
            const copyShortcut = (modifiers.platform && event.key === Qt.Key_C)
                || (modifiers.control && modifiers.shift && event.key === Qt.Key_C)
            const pasteShortcut = (modifiers.platform && event.key === Qt.Key_V)
                || (modifiers.control && modifiers.shift && event.key === Qt.Key_V)
            if (copyShortcut && root.copySelection()) {
                event.accepted = true
            } else if (pasteShortcut) {
                root.pasteFromClipboard()
                event.accepted = true
            } else if ((modifiers.platform && event.key === Qt.Key_A)
                    || (modifiers.control && modifiers.shift && event.key === Qt.Key_A)) {
                root.selectAll()
                event.accepted = true
            } else if (modifiers.shift && !modifiers.control
                    && !modifiers.alt && !modifiers.platform
                    && (event.key === Qt.Key_PageUp || event.key === Qt.Key_PageDown
                        || event.key === Qt.Key_Home || event.key === Qt.Key_End)) {
                const direction = event.key === Qt.Key_PageUp ? "pageUp"
                    : (event.key === Qt.Key_PageDown ? "pageDown"
                        : (event.key === Qt.Key_Home ? "top" : "bottom"))
                event.accepted = root.backend.scroll_terminal(direction)
            } else {
                const key = root.terminalKeyName(event)
                if (key.length > 0 && (modifiers.control || modifiers.alt
                        || key === "enter" || key === "tab" || key === "backspace"
                        || key === "escape" || key === "up" || key === "down"
                        || key === "left" || key === "right" || key === "home"
                        || key === "end" || key === "pageup" || key === "pagedown"
                        || key === "delete" || key === "insert" || key[0] === "f")) {
                    event.accepted = root.backend.send_terminal_key(
                        key, event.text, modifiers.shift, modifiers.control,
                        modifiers.alt, modifiers.platform)
                }
            }
        }
    }

    TextEdit {
        id: clipboardProxy
        x: -2
        y: -2
        width: 1
        height: 1
        opacity: 0
        readOnly: true
        textFormat: TextEdit.PlainText
    }

    Menu {
        id: contextMenu

        MenuItem { text: qsTr("Copy"); enabled: root.selectionAnchorRow >= 0; onTriggered: root.copySelection() }
        MenuItem { text: qsTr("Paste"); onTriggered: root.pasteFromClipboard() }
        MenuItem { text: qsTr("Select All"); onTriggered: root.selectAll() }
        MenuSeparator {}
        MenuItem { text: qsTr("Clear"); onTriggered: root.backend.clear_terminal_screen() }
    }

    Timer {
        id: resizeTimer
        interval: 40
        repeat: false
        onTriggered: {
            const rows = Math.floor(root.height / root.cellHeight)
            const columns = Math.floor(root.width / root.cellWidth)
            if (root.visible && rows >= 2 && columns >= 8)
                root.backend.resize_terminal(rows, columns)
        }
    }

    Timer {
        id: cursorTimer
        interval: 530
        repeat: true
        running: root.visible && terminalInput.activeFocus
            && root.backend.terminalCursorVisible
        onTriggered: root.cursorBlinkVisible = !root.cursorBlinkVisible
    }

    onObservedScreenRevisionChanged: {
        cursorBlinkVisible = true
        if (selecting)
            selectionInvalidated = true
        else
            resetSelection()
    }

    onWidthChanged: {
        if (visible)
            resizeTimer.restart()
    }
    onHeightChanged: {
        if (visible)
            resizeTimer.restart()
    }
    Component.onCompleted: {
        if (visible)
            resizeTimer.start()
    }
}
