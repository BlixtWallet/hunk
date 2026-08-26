import QtQuick
import QtTest

import "../../src/qml/Hunk"

Item {
    id: root
    width: 640
    height: 240

    Component {
        id: terminalRowComponent

        TerminalRow {
            width: 640
            row: 2
            lineMarkup: qsTr("terminal row")
            cellWidth: 8
            cellHeight: 16
            selectionAnchorRow: -1
            selectionAnchorColumn: -1
            selectionHeadRow: -1
            selectionHeadColumn: -1
        }
    }

    TestCase {
        name: "TerminalRowTests"
        when: windowShown

        function test_selectionDefaultsToInactive() {
            const row = createTemporaryObject(terminalRowComponent, root)
            verify(!!row, "Component exists")

            compare(row.hasSelection, false)
            compare(row.rowSelected, false)
            compare(row.height, 16)
        }

        function test_markupUsesTheSharedTerminalPalette() {
            const row = createTemporaryObject(terminalRowComponent, root, {
                lineMarkup: "@terminalRed@ text"
            })
            verify(!!row, "Component exists")

            verify(row.renderedMarkup.indexOf("@terminalRed@") < 0)
            verify(row.renderedMarkup.endsWith(" text"))
        }

        function test_forwardSelectionProjectsInclusiveColumns() {
            const row = createTemporaryObject(terminalRowComponent, root)
            verify(!!row, "Component exists")

            row.selectionAnchorRow = 2
            row.selectionAnchorColumn = 3
            row.selectionHeadRow = 2
            row.selectionHeadColumn = 7

            compare(row.selectionForward, true)
            compare(row.rowSelected, true)
            compare(row.selectedColumnStart, 3)
            compare(row.selectedColumnEnd, 7)
        }

        function test_reverseSelectionNormalizesRowsAndColumns() {
            const row = createTemporaryObject(terminalRowComponent, root)
            verify(!!row, "Component exists")

            row.selectionAnchorRow = 4
            row.selectionAnchorColumn = 9
            row.selectionHeadRow = 2
            row.selectionHeadColumn = 1

            compare(row.selectionForward, false)
            compare(row.firstSelectionRow, 2)
            compare(row.lastSelectionRow, 4)
            compare(row.selectedColumnStart, 1)
            compare(row.selectedColumnEnd, Math.floor(row.width / row.cellWidth) - 1)
        }
    }
}
