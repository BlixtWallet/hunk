pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property string label
    required property bool available
    required property int remainingPercent
    required property string resetLabel

    implicitHeight: 43
    Accessible.role: Accessible.StaticText
    Accessible.name: root.available
        ? qsTr("%1: %2 percent left. %3").arg(root.label).arg(root.remainingPercent).arg(root.resetLabel)
        : qsTr("%1: unavailable").arg(root.label)

    Text {
        anchors.left: parent.left
        anchors.top: parent.top
        text: root.label
        textFormat: Text.PlainText
        color: Theme.muted
        font.family: Theme.uiFont
        font.pixelSize: 11
    }

    Text {
        anchors.right: parent.right
        anchors.top: parent.top
        text: root.available
            ? qsTr("%1% left · %2").arg(root.remainingPercent).arg(root.resetLabel)
            : qsTr("Unavailable")
        textFormat: Text.PlainText
        color: root.available ? Theme.foreground : Theme.faint
        font.family: Theme.monoFont
        font.pixelSize: 10
    }

    Rectangle {
        height: 6
        radius: 3
        color: Theme.raised
        border.width: 1
        border.color: Theme.border
        Accessible.ignored: true
        anchors {
            left: parent.left
            right: parent.right
            bottom: parent.bottom
        }

        Rectangle {
            width: root.available ? parent.width * root.remainingPercent / 100 : 0
            height: parent.height
            radius: parent.radius
            color: root.remainingPercent <= 15 ? Theme.negative
                : (root.remainingPercent <= 35 ? Theme.warning : Theme.accent)
        }
    }
}
