pragma ComponentBehavior: Bound

import QtQuick

FocusScope {
    id: root

    required property var backend
    required property var draftStore
    property string currentThreadId: ""
    property bool restoringDraft: false
    property bool submitting: false
    property bool requestWasBlocking: false
    property bool queueWasBlocking: false
    property bool restoreFocusAfterRequest: false
    signal requestFocusRequested
    readonly property alias editor: editor
    readonly property alias sendButton: sendButton
    readonly property alias stopButton: stopButton
    readonly property bool requestBlocking: backend.aiRequestId.length > 0
        || backend.aiRequestResolving
    readonly property bool queueBlocking: backend.aiActiveQueueSending
        && !backend.aiTurnRunning
    readonly property bool editable: backend.aiReady && !backend.aiLoading
        && !backend.aiRequiresAuthentication && backend.aiActiveThreadId.length > 0
        && !backend.aiThreadActionPending
        && !backend.aiPromptPending && !backend.aiInterruptPending
        && !queueBlocking
        && !requestBlocking
    readonly property bool canSubmit: editable && editor.text.trim().length > 0

    implicitHeight: 142

    function draftEntry(threadId) {
        return draftStore[threadId] || {
            text: "",
            pending: false,
            acceptedRevision: backend.aiPromptAcceptedRevision
        }
    }

    function storeEntry(threadId, entry) {
        if (threadId.length > 0)
            draftStore[threadId] = entry
    }

    function saveCurrentDraft() {
        if (restoringDraft || currentThreadId.length === 0)
            return
        const entry = draftEntry(currentThreadId)
        entry.text = editor.text
        storeEntry(currentThreadId, entry)
    }

    function mergedPrompt(existing, recovered) {
        const current = existing.trim()
        const restored = recovered.trim()
        if (restored.length === 0)
            return existing
        if (current.length === 0)
            return restored
        if (current.replace(/\r\n/g, "\n") === restored.replace(/\r\n/g, "\n"))
            return existing
        return existing + "\n\n" + restored
    }

    function restoreRecoveredDraft(threadId) {
        if (threadId.length === 0)
            return
        const recovered = backend.take_ai_recovered_prompt(threadId)
        if (recovered.length === 0)
            return
        const entry = draftEntry(threadId)
        entry.text = mergedPrompt(entry.text, recovered)
        entry.pending = false
        storeEntry(threadId, entry)
        if (threadId === currentThreadId) {
            restoringDraft = true
            editor.text = entry.text
            restoringDraft = false
            submitting = false
            editorFocusTimer.restart()
        }
    }

    function reloadDraftStore() {
        currentThreadId = ""
        restoringDraft = true
        editor.text = ""
        restoringDraft = false
        activateThread(backend.aiActiveThreadId)
    }

    function activateThread(threadId) {
        if (threadId === currentThreadId)
            return
        saveCurrentDraft()
        currentThreadId = threadId
        restoreRecoveredDraft(threadId)
        restoringDraft = true
        const entry = draftEntry(threadId)
        if (entry.pending && backend.aiPromptAcceptedRevision !== entry.acceptedRevision) {
            entry.text = ""
            entry.pending = false
            storeEntry(threadId, entry)
        } else if (entry.pending && !backend.aiPromptPending) {
            entry.pending = false
            storeEntry(threadId, entry)
        }
        editor.text = entry.text
        submitting = entry.pending && backend.aiPromptPending
        restoringDraft = false
        if (editable && !submitting)
            editor.forceActiveFocus()
    }

    function syncBackendState() {
        activateThread(backend.aiActiveThreadId)
        restoreRecoveredDraft(currentThreadId)
        if (currentThreadId.length === 0) {
            requestWasBlocking = requestBlocking
            return
        }
        const entry = draftEntry(currentThreadId)
        if (entry.pending && backend.aiPromptAcceptedRevision !== entry.acceptedRevision) {
            entry.text = ""
            entry.pending = false
            storeEntry(currentThreadId, entry)
            restoringDraft = true
            editor.text = ""
            restoringDraft = false
            submitting = false
            editor.forceActiveFocus()
        } else if (entry.pending && !backend.aiPromptPending) {
            entry.pending = false
            storeEntry(currentThreadId, entry)
            submitting = false
            editor.forceActiveFocus()
        } else {
            submitting = entry.pending && backend.aiPromptPending
        }
        if (!requestWasBlocking && requestBlocking && restoreFocusAfterRequest)
            Qt.callLater(() => root.requestFocusRequested())
        if (requestWasBlocking && !requestBlocking && editable
                && restoreFocusAfterRequest)
            Qt.callLater(() => editor.forceActiveFocus())
        if (queueWasBlocking && !queueBlocking && editable)
            editorFocusTimer.restart()
        requestWasBlocking = requestBlocking
        queueWasBlocking = queueBlocking
    }

    function submit() {
        if (!canSubmit)
            return
        const prompt = editor.text
        const acceptedRevision = backend.aiPromptAcceptedRevision
        if (!backend.send_ai_prompt(prompt)) {
            editor.forceActiveFocus()
            return
        }
        storeEntry(currentThreadId, {
            text: prompt,
            pending: true,
            acceptedRevision: acceptedRevision
        })
        submitting = true
    }

    function queueFollowUp() {
        if (!backend.aiTurnRunning || !canSubmit)
            return
        const prompt = editor.text
        if (!backend.queue_ai_follow_up(prompt)) {
            editor.forceActiveFocus()
            return
        }
        storeEntry(currentThreadId, {
            text: "",
            pending: false,
            acceptedRevision: backend.aiPromptAcceptedRevision
        })
        restoringDraft = true
        editor.text = ""
        restoringDraft = false
        submitting = false
        editor.forceActiveFocus()
    }

    function editLatestQueuedFollowUp() {
        if (editor.text.length > 0)
            return
        const prompt = backend.edit_last_ai_queued_prompt()
        if (prompt.length === 0)
            return
        storeEntry(currentThreadId, {
            text: prompt,
            pending: false,
            acceptedRevision: backend.aiPromptAcceptedRevision
        })
        restoringDraft = true
        editor.text = prompt
        editor.cursorPosition = editor.text.length
        restoringDraft = false
        submitting = false
        editor.forceActiveFocus()
    }

    function interrupt() {
        if (backend.aiTurnRunning && !backend.aiThreadActionPending
                && !backend.aiInterruptPending)
            backend.interrupt_ai_turn()
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.canvas

        Rectangle {
            anchors.top: parent.top
            width: parent.width
            height: 1
            color: Theme.border
        }
    }

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        anchors.leftMargin: 20
        anchors.rightMargin: 20
        anchors.topMargin: 12
        anchors.bottomMargin: 12
        radius: Theme.radius
        color: Theme.input
        border.width: editor.activeFocus ? 1 : 0
        border.color: Theme.accentStrong

        Flickable {
            id: editorViewport
            anchors.left: parent.left
            anchors.right: actions.left
            anchors.top: parent.top
            anchors.bottom: footer.top
            anchors.leftMargin: 12
            anchors.rightMargin: 12
            anchors.topMargin: 10
            anchors.bottomMargin: 6
            clip: true
            contentWidth: width
            contentHeight: Math.max(height, editor.height)
            boundsBehavior: Flickable.StopAtBounds

            TextEdit {
                id: editor
                objectName: "aiPromptEditor"
                width: editorViewport.width
                height: Math.max(editorViewport.height, contentHeight)
                enabled: root.editable
                textFormat: TextEdit.PlainText
                color: Theme.foreground
                selectionColor: Theme.accent
                selectedTextColor: Theme.foreground
                wrapMode: TextEdit.Wrap
                selectByMouse: true
                font.family: Theme.uiFont
                font.pixelSize: 12

                onActiveFocusChanged: {
                    if (activeFocus && !root.requestBlocking)
                        root.restoreFocusAfterRequest = true
                    else if (!activeFocus && !root.requestBlocking
                            && !root.requestWasBlocking
                            && !root.backend.aiThreadActionPending)
                        root.restoreFocusAfterRequest = false
                }
                onTextChanged: root.saveCurrentDraft()
                onCursorRectangleChanged: {
                    if (cursorRectangle.y < editorViewport.contentY)
                        editorViewport.contentY = Math.max(0, cursorRectangle.y)
                    else if (cursorRectangle.y + cursorRectangle.height
                            > editorViewport.contentY + editorViewport.height)
                        editorViewport.contentY = cursorRectangle.y + cursorRectangle.height
                            - editorViewport.height
                }

                Keys.onPressed: event => {
                    if (event.key === Qt.Key_Tab
                            && event.modifiers === Qt.NoModifier
                            && root.backend.aiTurnRunning) {
                        root.queueFollowUp()
                        event.accepted = true
                        return
                    }
                    if (event.key === Qt.Key_Up
                            && (event.modifiers & Qt.ControlModifier) !== 0
                            && (event.modifiers & Qt.ShiftModifier) !== 0) {
                        root.editLatestQueuedFollowUp()
                        event.accepted = true
                        return
                    }
                    if (event.key !== Qt.Key_Return && event.key !== Qt.Key_Enter)
                        return
                    if ((event.modifiers & Qt.ShiftModifier) !== 0) {
                        editor.insert(editor.cursorPosition, "\n")
                    } else {
                        root.submit()
                    }
                    event.accepted = true
                }
            }

            Text {
                anchors.left: parent.left
                anchors.top: parent.top
                visible: editor.text.length === 0
                text: root.backend.aiActiveThreadId.length === 0
                    ? "Select or create a thread to begin"
                    : (root.backend.aiRequestId.length > 0
                        ? "Respond to Codex above to continue"
                        : (root.backend.aiTurnRunning
                            ? "Add instructions to the active turn…"
                            : "Ask Codex to work on this repository…"))
                color: Theme.faint
                font.family: Theme.uiFont
                font.pixelSize: 12
            }
        }

        Column {
            id: actions
            anchors.right: parent.right
            anchors.rightMargin: 10
            anchors.verticalCenter: parent.verticalCenter
            spacing: 6

            ActionButton {
                id: sendButton
                objectName: "aiSendButton"
                label: root.backend.aiTurnRunning ? "Steer" : "Send"
                primary: true
                compact: true
                enabled: root.canSubmit
                onClicked: root.submit()
            }

            ActionButton {
                id: stopButton
                objectName: "aiStopButton"
                label: root.backend.aiInterruptPending ? "Stopping" : "Stop"
                danger: true
                compact: true
                visible: root.backend.aiTurnRunning
                enabled: root.backend.aiReady && !root.backend.aiInterruptPending
                    && !root.backend.aiThreadActionPending
                    && !root.backend.aiPromptPending
                onClicked: root.interrupt()
            }
        }

        Row {
            id: footer
            anchors.left: parent.left
            anchors.right: actions.left
            anchors.bottom: parent.bottom
            anchors.leftMargin: 12
            anchors.rightMargin: 12
            anchors.bottomMargin: 8
            spacing: 8

            Text {
                text: root.backend.aiPromptPending ? "SENDING"
                    : (root.backend.aiInterruptPending ? "STOPPING"
                        : (root.backend.aiActiveQueuedMessageCount > 0
                            ? root.backend.aiActiveQueuedMessageCount + " QUEUED"
                        : (root.backend.aiRequestId.length > 0
                            ? "RESPONSE REQUIRED" : "ENTER TO SEND")))
                color: Theme.faint
                font.family: Theme.monoFont
                font.pixelSize: 8
                font.letterSpacing: 0.6
            }

            Text {
                text: root.backend.aiTurnRunning
                    ? "TAB TO QUEUE · CTRL+SHIFT+UP TO EDIT"
                    : "SHIFT+ENTER FOR NEW LINE"
                color: Theme.faint
                font.family: Theme.monoFont
                font.pixelSize: 8
                font.letterSpacing: 0.5
            }
        }
    }

    Connections {
        target: root.backend

        function onAiStateChanged() {
            root.syncBackendState()
        }
    }

    Timer {
        id: editorFocusTimer
        interval: 0
        repeat: false
        onTriggered: {
            if (root.editable)
                editor.forceActiveFocus()
        }
    }

    onDraftStoreChanged: {
        if (editor)
            reloadDraftStore()
    }
    Component.onCompleted: {
        requestWasBlocking = requestBlocking
        queueWasBlocking = queueBlocking
        activateThread(backend.aiActiveThreadId)
        if (editable && !submitting)
            editor.forceActiveFocus()
    }
}
