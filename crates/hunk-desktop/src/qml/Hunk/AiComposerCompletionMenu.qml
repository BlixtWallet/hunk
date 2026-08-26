pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    property var items: []
    property int selectedIndex: 0
    signal accepted(string kind, string value)
    signal hovered(int index)

    readonly property int rowHeight: 43
    implicitHeight: items.length * rowHeight + 2
    visible: items.length > 0
    Accessible.role: Accessible.List
    Accessible.name: qsTr("Composer completions")

    Rectangle {
        anchors.fill: parent
        radius: Theme.radius
        color: Theme.raised
        border.width: 1
        border.color: Theme.borderStrong
    }

    Column {
        anchors.fill: parent
        anchors.margins: 1

        Repeater {
            model: root.items

            delegate: Item {
                id: completionRow

                required property int index
                required property var modelData

                width: parent.width
                height: root.rowHeight
                opacity: modelData.disabled ? 0.48 : 1
                Accessible.role: Accessible.ListItem
                Accessible.name: modelData.label
                Accessible.description: modelData.description

                Rectangle {
                    anchors.fill: parent
                    color: completionRow.index === root.selectedIndex
                        ? Theme.selected : Theme.transparent
                }

                Text {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    anchors.topMargin: 6
                    text: completionRow.modelData.label
                    textFormat: Text.PlainText
                    color: Theme.foreground
                    elide: Text.ElideRight
                    font.family: completionRow.modelData.kind === "file"
                        ? Theme.monoFont : Theme.uiFont
                    font.pixelSize: 11
                    font.weight: Font.DemiBold
                }

                Text {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    anchors.leftMargin: 12
                    anchors.rightMargin: 12
                    anchors.bottomMargin: 6
                    text: completionRow.modelData.disabled
                        ? qsTr("Disabled while a task is in progress.")
                        : completionRow.modelData.description
                    textFormat: Text.PlainText
                    color: Theme.muted
                    elide: Text.ElideMiddle
                    font.family: Theme.uiFont
                    font.pixelSize: 9
                }

                MouseArea {
                    anchors.fill: parent
                    enabled: !completionRow.modelData.disabled
                    hoverEnabled: true
                    cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                    onEntered: root.hovered(completionRow.index)
                    onClicked: root.accepted(
                        completionRow.modelData.kind,
                        completionRow.modelData.value
                    )
                }
            }
        }
    }
}
