pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property string row_id
    required property string turn_id
    required property string kind
    required property string role
    required property string title
    required property string text
    required property string status
    required property bool streaming
    required property bool mono
    required property bool truncated
    required property double last_sequence

    readonly property bool userRow: role === "user"
    readonly property bool toolRow: role === "tool"
    readonly property bool systemRow: role === "system"
    readonly property alias bodyTextItem: bodyText

    implicitHeight: content.implicitHeight + (toolRow ? 20 : 28)

    Rectangle {
        anchors.fill: parent
        anchors.leftMargin: root.userRow ? 18 : 0
        anchors.rightMargin: root.userRow ? 0 : (root.toolRow ? 18 : 0)
        radius: root.userRow ? 7 : 0
        color: root.userRow ? Theme.accentMuted
            : (root.toolRow ? Theme.input : Theme.transparent)

        Rectangle {
            visible: root.userRow
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: 2
            radius: 1
            color: Theme.accentStrong
        }
    }

    Column {
        id: content
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: root.userRow ? 32 : (root.toolRow ? 14 : 4)
        anchors.rightMargin: root.userRow ? 14 : (root.toolRow ? 32 : 4)
        spacing: 7

        Row {
            width: parent.width
            spacing: 8

            Text {
                id: rowTitle
                width: Math.max(0, Math.min(implicitWidth, parent.width - rowStatus.width - 8))
                text: root.title
                textFormat: Text.PlainText
                color: root.userRow ? Theme.accentStrong
                    : (root.systemRow ? Theme.muted : Theme.foreground)
                elide: Text.ElideRight
                font.family: Theme.uiFont
                font.pixelSize: root.toolRow ? 10 : 11
                font.weight: Font.DemiBold
                font.capitalization: root.toolRow || root.systemRow
                    ? Font.AllUppercase : Font.MixedCase
                font.letterSpacing: root.toolRow || root.systemRow ? 0.7 : 0
            }

            Text {
                id: rowStatus
                anchors.verticalCenter: rowTitle.verticalCenter
                text: root.streaming ? (root.status || "streaming") : root.status
                textFormat: Text.PlainText
                visible: text.length > 0
                color: root.streaming ? Theme.warning : Theme.faint
                font.family: Theme.monoFont
                font.pixelSize: 9
                font.capitalization: Font.AllUppercase
            }
        }

        TextEdit {
            id: bodyText
            width: parent.width
            height: contentHeight
            text: root.text
            textFormat: TextEdit.PlainText
            color: root.systemRow ? Theme.muted : Theme.foreground
            selectionColor: Theme.accent
            selectedTextColor: Theme.foreground
            readOnly: true
            selectByMouse: true
            wrapMode: TextEdit.Wrap
            font.family: root.mono ? Theme.monoFont : Theme.uiFont
            font.pixelSize: root.mono ? 11 : 13
        }

        Text {
            visible: root.truncated
            text: "CONTENT TRUNCATED"
            color: Theme.faint
            font.family: Theme.monoFont
            font.pixelSize: 8
            font.letterSpacing: 0.7
        }
    }

    Rectangle {
        visible: !root.userRow && !root.toolRow
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 1
        color: Theme.border
        opacity: 0.45
    }
}
