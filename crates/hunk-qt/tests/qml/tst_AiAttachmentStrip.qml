pragma ComponentBehavior: Bound

import QtQuick
import QtTest
import Hunk

Item {
    id: root
    width: 700
    height: 220

    ListModel {
        id: attachmentModel
    }

    QtObject {
        id: fakeBackend

        property QtObject aiAttachments: attachmentModel
        readonly property int aiAttachmentCount: attachmentModel.count
        property bool aiModelSupportsImageInputs: true
        property string aiActiveThreadId: "thread-1"
        property bool attachmentsEditable: true
        property string lastPathsJson: ""
        property int addCallCount: 0
        property int removedIndex: -1

        function add_ai_attachments(pathsJson) {
            lastPathsJson = pathsJson
            addCallCount += 1
            const paths = JSON.parse(pathsJson)
            for (let index = 0; index < paths.length; ++index) {
                const path = paths[index]
                const parts = path.split("/")
                attachmentModel.append({
                    path: path,
                    display_name: parts[parts.length - 1]
                })
            }
            return paths.length > 0
        }

        function remove_ai_attachment(index) {
            if (index < 0 || index >= attachmentModel.count)
                return false
            removedIndex = index
            attachmentModel.remove(index)
            return true
        }
    }

    Component {
        id: attachmentStripComponent

        AiAttachmentStrip {
            width: 640
            height: implicitHeight
            backend: fakeBackend
            editable: fakeBackend.attachmentsEditable
        }
    }

    TestCase {
        name: "AiAttachmentStripTests"
        when: windowShown

        function init() {
            attachmentModel.clear()
            fakeBackend.aiModelSupportsImageInputs = true
            fakeBackend.attachmentsEditable = true
            fakeBackend.lastPathsJson = ""
            fakeBackend.addCallCount = 0
            fakeBackend.removedIndex = -1
        }

        function test_addUrlsRoutesTheSelectedFiles() {
            const strip = createTemporaryObject(attachmentStripComponent, root)
            verify(!!strip, "Component exists")

            strip.addUrls([
                "first.png",
                "second.webp"
            ])

            compare(fakeBackend.addCallCount, 1)
            compare(JSON.parse(fakeBackend.lastPathsJson).length, 2)
            compare(attachmentModel.count, 2)
            compare(attachmentModel.get(0).display_name, "first.png")
        }

        function test_addUrlsRejectsOversizedCandidateListsBeforeCallingRust() {
            const strip = createTemporaryObject(attachmentStripComponent, root)
            verify(!!strip, "Component exists")
            const paths = []
            for (let index = 0; index <= strip.maxCandidateCount; ++index)
                paths.push("capture-" + index + ".png")

            verify(!strip.addUrls(paths))

            compare(fakeBackend.addCallCount, 0)
            verify(strip.selectionError.length > 0)
            const warning = findChild(strip, "aiAttachmentWarning")
            verify(!!warning, "Object exists")
            compare(warning.visible, true)
        }

        function test_removeActionUpdatesTheDraft() {
            attachmentModel.append({
                path: "remove.png",
                display_name: "remove.png"
            })
            const strip = createTemporaryObject(attachmentStripComponent, root)
            verify(!!strip, "Component exists")
            const list = findChild(strip, "aiAttachmentList")
            verify(!!list, "Object exists")
            tryCompare(list, "count", 1)
            tryVerify(() => !!findChild(strip, "aiRemoveAttachmentButton-0"))
            const removeButton = findChild(strip, "aiRemoveAttachmentButton-0")
            verify(!!removeButton, "Object exists")

            mouseClick(removeButton)

            tryCompare(attachmentModel, "count", 0)
            tryCompare(fakeBackend, "removedIndex", 0)
        }

        function test_incompatibleModelShowsTheWarning() {
            fakeBackend.aiModelSupportsImageInputs = false
            attachmentModel.append({
                path: "unsupported.png",
                display_name: "unsupported.png"
            })
            const strip = createTemporaryObject(attachmentStripComponent, root)
            verify(!!strip, "Component exists")
            const warning = findChild(strip, "aiAttachmentWarning")
            verify(!!warning, "Object exists")

            compare(strip.modelSupportsAttachments, false)
            compare(warning.visible, true)
        }

        function test_pendingPromptDisablesRemoval() {
            fakeBackend.attachmentsEditable = false
            attachmentModel.append({
                path: "pending.png",
                display_name: "pending.png"
            })
            const strip = createTemporaryObject(attachmentStripComponent, root)
            verify(!!strip, "Component exists")
            tryVerify(() => !!findChild(strip, "aiRemoveAttachmentButton-0"))
            const removeButton = findChild(strip, "aiRemoveAttachmentButton-0")
            verify(!!removeButton, "Object exists")

            compare(removeButton.enabled, false)
            compare(attachmentModel.count, 1)
        }
    }
}
