pragma ComponentBehavior: Bound

import QtQuick

FocusScope {
    id: root

    required property var backend
    required property int row
    required property string lineHint
    property alias text: editor.text
    readonly property alias editor: editor
    property bool submitting: false
    signal cancelled
    signal saved

    implicitWidth: 380
    implicitHeight: 126

    function submit() {
        if (submitting || editor.text.trim().length === 0)
            return
        submitting = true
        backend.create_diff_comment(row, editor.text)
    }

    function reset() {
        submitting = false
        editor.text = ""
    }

    Rectangle {
        anchors.fill: parent
        radius: 7
        color: Theme.raised
        border.width: 1
        border.color: Theme.borderStrong
    }

    Column {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 7

        Row {
            width: parent.width
            spacing: 8

            Text {
                width: parent.width - location.implicitWidth - 8
                text: "Add review comment"
                color: Theme.foreground
                elide: Text.ElideRight
                font.family: Theme.uiFont
                font.pixelSize: 11
                font.weight: Font.DemiBold
            }

            Text {
                id: location
                text: root.lineHint
                color: Theme.faint
                font.family: Theme.monoFont
                font.pixelSize: 8
            }
        }

        Rectangle {
            width: parent.width
            height: 58
            radius: 5
            color: Theme.input
            border.width: editor.activeFocus ? 1 : 0
            border.color: Theme.accentStrong

            TextEdit {
                id: editor
                objectName: "diffCommentEditor"
                anchors.fill: parent
                anchors.margins: 7
                enabled: !root.submitting
                color: Theme.foreground
                selectionColor: Theme.accent
                selectedTextColor: Theme.foreground
                wrapMode: TextEdit.Wrap
                selectByMouse: true
                font.family: Theme.uiFont
                font.pixelSize: 11

                Keys.onPressed: event => {
                    const commandModifier = (event.modifiers
                        & (Qt.ControlModifier | Qt.MetaModifier)) !== 0
                    if (commandModifier
                            && (event.key === Qt.Key_Return || event.key === Qt.Key_Enter)) {
                        root.submit()
                        event.accepted = true
                    } else if (event.key === Qt.Key_Escape) {
                        root.cancelled()
                        event.accepted = true
                    }
                }
            }

            Text {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.margins: 7
                visible: editor.text.length === 0 && !editor.activeFocus
                text: "What should change here?"
                color: Theme.faint
                font.family: Theme.uiFont
                font.pixelSize: 11
            }
        }

        Row {
            anchors.right: parent.right
            spacing: 6

            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: root.submitting
                text: "SAVING"
                color: Theme.faint
                font.family: Theme.monoFont
                font.pixelSize: 8
                font.letterSpacing: 0.7
            }

            ActionButton {
                label: "Cancel"
                compact: true
                enabled: !root.submitting
                onClicked: root.cancelled()
            }

            ActionButton {
                label: "Save"
                compact: true
                primary: true
                enabled: !root.submitting && editor.text.trim().length > 0
                onClicked: root.submit()
            }
        }
    }

    Connections {
        target: root.backend
        enabled: root.submitting

        function onDiffCommentsStateChanged() {
            if (root.backend.diffCommentsBusy || root.backend.diffCommentsLoading)
                return
            if (root.backend.diffCommentsError.length > 0) {
                root.submitting = false
                editor.forceActiveFocus()
            } else if (root.backend.diffCommentsStatusMessage === "Comment added.") {
                root.submitting = false
                root.saved()
            }
        }
    }

    Component.onCompleted: editor.forceActiveFocus()
}
