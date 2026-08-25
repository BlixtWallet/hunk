pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property int row
    required property string lineMarkup
    required property real cellWidth
    required property real cellHeight
    required property int selectionAnchorRow
    required property int selectionAnchorColumn
    required property int selectionHeadRow
    required property int selectionHeadColumn
    readonly property string renderedMarkup: Theme.terminalMarkup(lineMarkup)

    readonly property bool hasSelection: selectionAnchorRow >= 0
        && selectionHeadRow >= 0
        && (selectionAnchorRow !== selectionHeadRow
            || selectionAnchorColumn !== selectionHeadColumn)
    readonly property bool selectionForward: selectionAnchorRow < selectionHeadRow
        || (selectionAnchorRow === selectionHeadRow
            && selectionAnchorColumn <= selectionHeadColumn)
    readonly property int firstSelectionRow: selectionForward
        ? selectionAnchorRow : selectionHeadRow
    readonly property int lastSelectionRow: selectionForward
        ? selectionHeadRow : selectionAnchorRow
    readonly property int firstSelectionColumn: selectionForward
        ? selectionAnchorColumn : selectionHeadColumn
    readonly property int lastSelectionColumn: selectionForward
        ? selectionHeadColumn : selectionAnchorColumn
    readonly property bool rowSelected: hasSelection
        && row >= firstSelectionRow && row <= lastSelectionRow
    readonly property int selectedColumnStart: !rowSelected ? 0
        : (row === firstSelectionRow ? firstSelectionColumn : 0)
    readonly property int selectedColumnEnd: !rowSelected ? -1
        : (row === lastSelectionRow ? lastSelectionColumn
            : Math.max(0, Math.floor(width / cellWidth) - 1))

    height: cellHeight

    Rectangle {
        visible: root.rowSelected && root.selectedColumnEnd >= root.selectedColumnStart
        x: root.selectedColumnStart * root.cellWidth
        width: (root.selectedColumnEnd - root.selectedColumnStart + 1) * root.cellWidth
        height: parent.height
        color: Theme.accentMuted
    }

    Text {
        anchors.left: parent.left
        anchors.top: parent.top
        width: parent.width
        height: parent.height
        text: root.renderedMarkup
        textFormat: Text.RichText
        color: Theme.terminalForeground
        wrapMode: Text.NoWrap
        verticalAlignment: Text.AlignVCenter
        font.family: Theme.monoFont
        font.pixelSize: 12
        font.kerning: false
        font.preferShaping: false
    }

}
