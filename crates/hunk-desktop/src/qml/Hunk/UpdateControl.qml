pragma ComponentBehavior: Bound

import QtQuick

Row {
    id: root

    required property QtObject updates
    signal restartRequested

    spacing: 8

    Text {
        objectName: "updateStatus"
        anchors.verticalCenter: parent.verticalCenter
        visible: root.updates.busy || root.updates.status === "error"
            || root.updates.status === "disabled"
        width: Math.min(180, implicitWidth)
        text: root.updates.statusMessage
        textFormat: Text.PlainText
        color: root.updates.status === "error" ? Theme.negative : Theme.muted
        elide: Text.ElideRight
        font.family: Theme.uiFont
        font.pixelSize: 11
    }

    ActionButton {
        objectName: "updateAction"
        anchors.verticalCenter: parent.verticalCenter
        label: root.updates.readyToRestart ? qsTr("Restart to update")
            : (root.updates.busy ? qsTr("Updating…")
                : (root.updates.status === "up_to_date"
                    ? qsTr("Up to date") : qsTr("Updates")))
        accessibleName: root.updates.readyToRestart
            ? qsTr("Restart Hunk to install version %1").arg(root.updates.version)
            : qsTr("Check for Hunk updates")
        compact: true
        primary: root.updates.readyToRestart
        enabled: root.updates.enabled && !root.updates.busy
        onClicked: {
            if (root.updates.readyToRestart)
                root.restartRequested()
            else
                root.updates.check_for_updates()
        }
    }
}
