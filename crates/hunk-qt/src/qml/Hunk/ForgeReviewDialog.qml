import QtQuick

FocusScope {
    id: root

    property string providerLabel: "Forge"
    property string reviewKindLabel: "Review"
    readonly property alias titleInput: titleField
    readonly property alias targetInput: targetField
    readonly property alias bodyInput: bodyField
    property bool draft: false
    signal submitted(string targetBranch, string title, string body, bool draft)
    signal rejected

    z: 100

    function prepare(defaultTarget, defaultTitle) {
        targetField.text = defaultTarget
        titleField.text = defaultTitle
        bodyField.text = ""
        draft = false
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
        width: Math.min(560, parent.width - 48)
        height: reviewContent.implicitHeight + 40
        radius: Theme.radius
        color: Theme.chrome
        border.width: 1
        border.color: Theme.borderStrong

        Column {
            id: reviewContent
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.margins: 20
            spacing: 10

            Text {
                text: root.providerLabel + " " + root.reviewKindLabel
                color: Theme.foreground
                font.family: Theme.uiFont
                font.pixelSize: 15
                font.weight: Font.DemiBold
            }

            Text {
                text: "TITLE"
                color: Theme.muted
                font.family: Theme.uiFont
                font.pixelSize: 9
                font.weight: Font.DemiBold
                font.letterSpacing: 0.8
            }

            Rectangle {
                width: parent.width
                height: 38
                radius: 5
                color: Theme.input
                border.width: titleField.activeFocus ? 1 : 0
                border.color: Theme.accentStrong

                TextInput {
                    id: titleField
                    objectName: "forgeReviewTitle"
                    anchors.fill: parent
                    anchors.margins: 10
                    color: Theme.foreground
                    selectionColor: Theme.accent
                    selectedTextColor: Theme.foreground
                    clip: true
                    font.family: Theme.uiFont
                    font.pixelSize: 12
                }
            }

            Text {
                text: "BASE BRANCH"
                color: Theme.muted
                font.family: Theme.uiFont
                font.pixelSize: 9
                font.weight: Font.DemiBold
                font.letterSpacing: 0.8
            }

            Rectangle {
                width: parent.width
                height: 38
                radius: 5
                color: Theme.input
                border.width: targetField.activeFocus ? 1 : 0
                border.color: Theme.accentStrong

                TextInput {
                    id: targetField
                    objectName: "forgeReviewTarget"
                    anchors.fill: parent
                    anchors.margins: 10
                    color: Theme.foreground
                    selectionColor: Theme.accent
                    selectedTextColor: Theme.foreground
                    clip: true
                    font.family: Theme.monoFont
                    font.pixelSize: 11
                }
            }

            Text {
                text: "DESCRIPTION"
                color: Theme.muted
                font.family: Theme.uiFont
                font.pixelSize: 9
                font.weight: Font.DemiBold
                font.letterSpacing: 0.8
            }

            Rectangle {
                width: parent.width
                height: 112
                radius: 5
                color: Theme.input
                border.width: bodyField.activeFocus ? 1 : 0
                border.color: Theme.accentStrong

                TextEdit {
                    id: bodyField
                    objectName: "forgeReviewBody"
                    anchors.fill: parent
                    anchors.margins: 10
                    color: Theme.foreground
                    selectionColor: Theme.accent
                    selectedTextColor: Theme.foreground
                    wrapMode: TextEdit.Wrap
                    selectByMouse: true
                    font.family: Theme.uiFont
                    font.pixelSize: 11
                }
            }

            Row {
                width: parent.width

                Item {
                    width: parent.width - reviewActions.width
                    height: 30

                    Rectangle {
                        anchors.left: parent.left
                        anchors.verticalCenter: parent.verticalCenter
                        width: 16
                        height: 16
                        radius: 3
                        color: root.draft ? Theme.accent : Theme.input
                        border.width: 1
                        border.color: root.draft ? Theme.accentStrong : Theme.borderStrong

                        Text {
                            anchors.centerIn: parent
                            text: root.draft ? "✓" : ""
                            color: Theme.foreground
                            font.pixelSize: 10
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.draft = !root.draft
                        }
                    }

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 23
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Create as draft"
                        color: Theme.muted
                        font.family: Theme.uiFont
                        font.pixelSize: 11
                    }
                }

                Row {
                    id: reviewActions
                    spacing: 8

                    ActionButton {
                        label: "Cancel"
                        onClicked: root.rejected()
                    }

                    ActionButton {
                        label: "Find / Create"
                        primary: true
                        enabled: targetField.text.trim().length > 0
                            && titleField.text.trim().length > 0
                        onClicked: root.submitted(
                            targetField.text,
                            titleField.text,
                            bodyField.text,
                            root.draft
                        )
                    }
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
            titleField.forceActiveFocus()
    }
}
