import QtQuick

FocusScope {
    id: root

    property string title: "Confirm action"
    property string message: ""
    property string confirmLabel: "Confirm"
    property Item previousFocusItem: null
    signal accepted
    signal rejected
    signal focusRestorationFailed

    visible: false
    z: 100

    Rectangle {
        anchors.fill: parent
        color: Theme.overlay

        MouseArea {
            anchors.fill: parent
            onClicked: root.rejected()
        }
    }

    Rectangle {
        anchors.centerIn: parent
        width: Math.min(440, parent.width - 48)
        height: dialogContent.implicitHeight + 40
        radius: Theme.radius
        color: Theme.chrome
        border.width: 1
        border.color: Theme.borderStrong

        Column {
            id: dialogContent
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.margins: 20
            spacing: 12

            Text {
                width: parent.width
                text: root.title
                textFormat: Text.PlainText
                color: Theme.foreground
                font.family: Theme.uiFont
                font.pixelSize: 15
                font.weight: Font.DemiBold
            }

            Text {
                width: parent.width
                text: root.message
                textFormat: Text.PlainText
                color: Theme.muted
                font.family: Theme.uiFont
                font.pixelSize: 12
                lineHeight: 1.25
                wrapMode: Text.WordWrap
            }

            Row {
                anchors.right: parent.right
                spacing: 8

                ActionButton {
                    label: "Cancel"
                    onClicked: root.rejected()
                }

                ActionButton {
                    label: root.confirmLabel
                    danger: true
                    onClicked: root.accepted()
                }
            }
        }
    }

    Keys.onEscapePressed: event => {
        root.rejected()
        event.accepted = true
    }

    onVisibleChanged: {
        if (visible) {
            const hostWindow = root.Window.window
            root.previousFocusItem = hostWindow === null ? null : hostWindow.activeFocusItem
            forceActiveFocus()
        } else {
            const restoreTarget = root.previousFocusItem
            root.previousFocusItem = null
            Qt.callLater(() => {
                if (restoreTarget !== null && restoreTarget.visible && restoreTarget.enabled) {
                    restoreTarget.forceActiveFocus()
                } else {
                    root.focusRestorationFailed()
                }
            })
        }
    }
}
