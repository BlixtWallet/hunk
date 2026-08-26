import QtQuick

Item {
    id: root

    property string label: ""
    property string accessibleName: label
    property bool primary: false
    property bool danger: false
    property bool compact: false
    property real maximumWidth: 1000000
    property int labelElide: Text.ElideNone
    signal clicked

    implicitWidth: Math.min(labelText.implicitWidth + (compact ? 18 : 24), maximumWidth)
    implicitHeight: compact ? 26 : 30
    opacity: enabled ? 1 : 0.42
    activeFocusOnTab: enabled
    Accessible.role: Accessible.Button
    Accessible.name: accessibleName
    Accessible.onPressAction: {
        if (root.enabled)
            root.clicked();
    }

    Rectangle {
        anchors.fill: parent
        radius: 5
        color: {
            if (root.danger)
                return pointer.containsMouse ? Theme.negativeMuted : Theme.transparent;
            if (root.primary)
                return pointer.containsMouse ? Theme.accentStrong : Theme.accent;
            return pointer.containsMouse ? Theme.hover : Theme.raised;
        }
        border.width: root.primary ? 0 : 1
        border.color: root.danger ? Theme.negative : Theme.border
    }

    Rectangle {
        anchors.fill: parent
        radius: 5
        color: Theme.transparent
        border.width: root.activeFocus ? 1 : 0
        border.color: Theme.accentStrong
    }

    Text {
        id: labelText
        anchors.centerIn: parent
        width: Math.max(0, parent.width - (root.compact ? 18 : 24))
        text: root.label
        textFormat: Text.PlainText
        color: root.danger ? Theme.negative : Theme.foreground
        elide: root.labelElide
        horizontalAlignment: Text.AlignHCenter
        font {
            family: Theme.uiFont
            pixelSize: root.compact ? 11 : 12
            weight: Font.DemiBold
        }
    }

    MouseArea {
        id: pointer
        anchors.fill: parent
        enabled: root.enabled
        hoverEnabled: true
        cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: root.clicked()
    }

    Keys.onReturnPressed: event => {
        root.clicked();
        event.accepted = true;
    }
    Keys.onSpacePressed: event => {
        root.clicked();
        event.accepted = true;
    }
}
