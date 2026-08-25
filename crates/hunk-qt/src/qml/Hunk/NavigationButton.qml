import QtQuick

Item {
    id: root

    required property string label
    required property string workspace
    property bool selected: false
    signal activated(string workspace)

    implicitWidth: labelText.implicitWidth + 28
    implicitHeight: Theme.headerHeight

    Rectangle {
        anchors.fill: parent
        color: pointer.containsMouse ? Theme.hover : "transparent"

        Behavior on color {
            ColorAnimation { duration: Theme.transitionDuration }
        }
    }

    Text {
        id: labelText
        anchors.centerIn: parent
        text: root.label
        color: root.selected ? Theme.foreground : Theme.muted
        font.family: Theme.uiFont
        font.pixelSize: 13
        font.weight: root.selected ? Font.DemiBold : Font.Medium
    }

    Rectangle {
        anchors.bottom: parent.bottom
        anchors.horizontalCenter: parent.horizontalCenter
        width: root.selected ? root.width - 18 : 0
        height: 2
        radius: 1
        color: Theme.accentStrong

        Behavior on width {
            NumberAnimation { duration: Theme.transitionDuration; easing.type: Easing.OutCubic }
        }
    }

    MouseArea {
        id: pointer
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.activated(root.workspace)
    }
}
