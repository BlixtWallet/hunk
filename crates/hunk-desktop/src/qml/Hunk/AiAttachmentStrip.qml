pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Dialogs

Item {
    id: root

    required property var backend
    required property bool editable
    property string selectionError: ""
    property string pickerThreadId: ""
    readonly property bool hasAttachments: attachmentList.count > 0
    readonly property bool modelSupportsAttachments: backend.aiModelSupportsImageInputs
    readonly property int maxCandidateCount: 64
    readonly property int maxPathCharacters: 32 * 1024
    readonly property int maxSelectionBytes: 256 * 1024
    signal interactionCompleted

    implicitHeight: attachmentList.height + (warningLabel.visible ? warningLabel.height + 5 : 0)

    function addUrls(urls) {
        selectionError = ""
        if (urls.length > maxCandidateCount) {
            selectionError = qsTr("Select at most %1 images at once.").arg(maxCandidateCount)
            return false
        }
        const values = []
        let selectionBytes = 2
        for (let index = 0; index < urls.length; index++) {
            const value = urls[index].toString()
            if (value.length > maxPathCharacters) {
                selectionError = qsTr("The attachment selection contains a path that is too long.")
                return false
            }
            selectionBytes += JSON.stringify(value).length * 3 + 1
            if (selectionBytes > maxSelectionBytes) {
                selectionError = qsTr("The attachment selection is too large.")
                return false
            }
            values.push(value)
        }
        return values.length > 0
            && backend.add_ai_attachments(JSON.stringify(values))
    }

    function openPicker() {
        if (editable && modelSupportsAttachments) {
            selectionError = ""
            pickerThreadId = backend.aiActiveThreadId
            attachmentDialog.open()
        }
    }

    ListView {
        id: attachmentList
        objectName: "aiAttachmentList"
        height: root.hasAttachments ? 28 : 0
        visible: root.hasAttachments
        orientation: ListView.Horizontal
        spacing: 6
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        model: root.backend.aiAttachments
        reuseItems: true
        cacheBuffer: Math.max(0, width)
        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
        }

        delegate: Rectangle {
            id: attachmentChip

            required property int index
            required property string display_name

            width: Math.min(210, attachmentLabel.implicitWidth + removeButton.implicitWidth + 30)
            height: 28
            radius: 6
            color: Theme.raised
            border.width: 1
            border.color: Theme.border

            Text {
                id: attachmentLabel
                text: attachmentChip.display_name
                textFormat: Text.PlainText
                color: Theme.muted
                elide: Text.ElideMiddle
                font {
                    family: Theme.uiFont
                    pixelSize: 10
                }
                anchors {
                    left: parent.left
                    right: removeButton.left
                    leftMargin: 9
                    rightMargin: 5
                    verticalCenter: parent.verticalCenter
                }
            }

            ActionButton {
                id: removeButton
                objectName: "aiRemoveAttachmentButton-" + attachmentChip.index
                label: qsTr("×")
                accessibleName: qsTr("Remove %1").arg(attachmentChip.display_name)
                compact: true
                enabled: root.editable
                onClicked: {
                    if (root.backend.remove_ai_attachment(attachmentChip.index))
                        root.interactionCompleted()
                }
                anchors {
                    right: parent.right
                    rightMargin: 3
                    verticalCenter: parent.verticalCenter
                }
            }
        }
    }

    Text {
        id: warningLabel
        objectName: "aiAttachmentWarning"
        visible: root.selectionError.length > 0
            || (root.hasAttachments && !root.modelSupportsAttachments)
        text: root.selectionError.length > 0 ? root.selectionError
            : qsTr("Selected model does not support image attachments. Remove them or switch models.")
        textFormat: Text.PlainText
        color: Theme.warning
        elide: Text.ElideRight
        font {
            family: Theme.uiFont
            pixelSize: 9
        }
        anchors {
            left: parent.left
            right: parent.right
            top: attachmentList.bottom
            topMargin: 5
        }
    }

    FileDialog {
        id: attachmentDialog
        objectName: "aiAttachmentDialog"
        title: qsTr("Attach images")
        fileMode: FileDialog.OpenFiles
        nameFilters: [
            qsTr("Images (*.png *.jpg *.jpeg *.webp *.bmp *.gif *.tif *.tiff)"),
            qsTr("All files (*)")
        ]
        onAccepted: {
            if (root.pickerThreadId !== root.backend.aiActiveThreadId) {
                root.selectionError = qsTr("The active thread changed. Choose the images again.")
            } else {
                root.addUrls(selectedFiles)
            }
            root.pickerThreadId = ""
            root.interactionCompleted()
        }
        onRejected: {
            root.pickerThreadId = ""
            root.interactionCompleted()
        }
    }
}
