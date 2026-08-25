import QtQuick

FocusScope {
    id: root

    property string providerLabel: "Forge"
    readonly property alias tokenInput: tokenField
    signal submitted(string token)
    signal rejected

    z: 100

    function prepare() {
        tokenField.text = ""
    }

    function clear() {
        tokenField.text = ""
    }

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
        width: Math.min(460, parent.width - 48)
        height: tokenContent.implicitHeight + 40
        radius: Theme.radius
        color: Theme.chrome
        border.width: 1
        border.color: Theme.borderStrong

        Column {
            id: tokenContent
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.margins: 20
            spacing: 12

            Text {
                text: "Connect " + root.providerLabel
                color: Theme.foreground
                font.family: Theme.uiFont
                font.pixelSize: 15
                font.weight: Font.DemiBold
            }

            Text {
                width: parent.width
                text: "The token is sent directly to Rust and saved in the system credential store. It is never persisted in QML or the Hunk config file."
                color: Theme.muted
                wrapMode: Text.WordWrap
                lineHeight: 1.25
                font.family: Theme.uiFont
                font.pixelSize: 11
            }

            Rectangle {
                width: parent.width
                height: 38
                radius: 5
                color: Theme.input
                border.width: tokenField.activeFocus ? 1 : 0
                border.color: Theme.accentStrong

                TextInput {
                    id: tokenField
                    objectName: "forgeTokenInput"
                    anchors.fill: parent
                    anchors.margins: 10
                    echoMode: TextInput.Password
                    color: Theme.foreground
                    selectionColor: Theme.accent
                    selectedTextColor: Theme.foreground
                    clip: true
                    font.family: Theme.monoFont
                    font.pixelSize: 11
                    Keys.onReturnPressed: root.submitted(text)
                }
            }

            Row {
                anchors.right: parent.right
                spacing: 8

                ActionButton {
                    label: "Cancel"
                    onClicked: root.rejected()
                }

                ActionButton {
                    label: "Save token"
                    primary: true
                    enabled: tokenField.text.trim().length > 0
                    onClicked: root.submitted(tokenField.text)
                }
            }
        }
    }

    Keys.onEscapePressed: event => {
        root.rejected()
        event.accepted = true
    }

    onVisibleChanged: {
        if (visible)
            tokenField.forceActiveFocus()
    }
}
