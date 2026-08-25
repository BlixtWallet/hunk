pragma ComponentBehavior: Bound

import QtQuick

FocusScope {
    id: root

    required property var backend
    required property var answerStore
    property string loadedRequestId: ""
    property var questions: []
    property var answers: ({})
    property bool parseError: false
    property bool responseWasAvailable: false
    readonly property bool approvalRequest: backend.aiRequestKind === "approval"
    readonly property bool inputRequest: backend.aiRequestKind === "userInput"
    readonly property bool responseBusy: backend.aiRequestResolving
    readonly property bool interactionBlocked: backend.aiThreadActionPending
        || responseBusy
    readonly property bool hasRequest: backend.aiRequestId.length > 0
    readonly property bool canRespond: backend.aiReady && !backend.aiLoading
        && !backend.aiRequiresAuthentication && backend.aiRequestAnswerable
        && !interactionBlocked && loadedRequestId.length > 0 && !parseError
    readonly property alias acceptButton: acceptAction
    readonly property alias declineButton: declineAction
    readonly property alias submitButton: submitAction
    readonly property alias requestViewport: viewport

    visible: hasRequest
    implicitHeight: hasRequest
        ? Math.min(310, Math.max(154, contentColumn.height + 32))
        : 0

    function copyAnswers() {
        const copy = {}
        for (const key in answers)
            copy[key] = answers[key]
        return copy
    }

    function answerFor(questionId) {
        const value = answers[questionId]
        return value === undefined ? "" : value
    }

    function selectedOptionDescription(question) {
        const options = Array.isArray(question.options) ? question.options : []
        const answer = answerFor(question.id)
        for (const option of options) {
            if (option.label === answer)
                return option.description || ""
        }
        return ""
    }

    function setAnswer(questionId, answer) {
        if (!inputRequest || interactionBlocked)
            return
        const next = copyAnswers()
        next[questionId] = answer
        answers = next
        answerStore[loadedRequestId] = next
    }

    function answerCanBeRestored(question, answer) {
        if (typeof answer !== "string")
            return false
        const options = Array.isArray(question.options) ? question.options : []
        return options.length === 0 || question.isOther
            || options.some(option => option.label === answer)
    }

    function serializedAnswers() {
        const payload = {}
        for (const key in answers)
            payload[key] = [answers[key]]
        return JSON.stringify(payload)
    }

    function syncRequest() {
        const requestId = backend.aiRequestId
        if (requestId === loadedRequestId)
            return

        loadedRequestId = requestId
        questions = []
        answers = ({})
        parseError = false
        if (requestId.length === 0)
            return

        try {
            const parsed = JSON.parse(backend.aiRequestQuestionsJson)
            if (!Array.isArray(parsed))
                throw new Error("Questions are not an array")
            questions = parsed
            const initialAnswers = {}
            const retainedAnswers = answerStore[requestId] || {}
            for (const question of parsed) {
                const options = Array.isArray(question.options) ? question.options : []
                const retainedAnswer = retainedAnswers[question.id]
                initialAnswers[question.id] = answerCanBeRestored(
                    question, retainedAnswer)
                    ? retainedAnswer
                    : (options.length > 0 ? options[0].label : "")
            }
            answers = initialAnswers
            answerStore[requestId] = initialAnswers
        } catch (error) {
            parseError = true
        }
    }

    function focusFirstControl() {
        if (loadedRequestId.length === 0 || !canRespond)
            return false
        if (approvalRequest) {
            acceptAction.forceActiveFocus()
            return acceptAction.activeFocus
        }
        const firstAnswer = root.nextItemInFocusChain(true)
        if (firstAnswer && firstAnswer !== root) {
            firstAnswer.forceActiveFocus()
            return firstAnswer.activeFocus
        }
        if (submitAction.enabled) {
            submitAction.forceActiveFocus()
            return submitAction.activeFocus
        }
        return false
    }

    function syncBackendState() {
        syncRequest()
        const focusNewlyAvailableResponse = canRespond && !responseWasAvailable
        responseWasAvailable = canRespond
        if (focusNewlyAvailableResponse)
            responseFocusTimer.restart()
    }

    function resolveApproval(accept) {
        if (!approvalRequest || !canRespond)
            return false
        return backend.resolve_ai_approval(loadedRequestId, accept)
    }

    function submitInput() {
        if (!inputRequest || !canRespond)
            return false
        return backend.submit_ai_user_input(loadedRequestId, serializedAnswers())
    }

    Timer {
        id: responseFocusTimer
        interval: 0
        onTriggered: root.focusFirstControl()
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.raised

        Rectangle {
            anchors.top: parent.top
            width: parent.width
            height: 1
            color: Theme.accentStrong
        }

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.border
        }
    }

    Flickable {
        id: viewport
        anchors.fill: parent
        anchors.leftMargin: 20
        anchors.rightMargin: 20
        anchors.topMargin: 14
        anchors.bottomMargin: 14
        contentWidth: width
        contentHeight: contentColumn.height
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        function reveal(item) {
            if (!item || contentHeight <= height)
                return
            const position = item.mapToItem(contentColumn, 0, 0)
            const top = position.y
            const bottom = top + item.height
            if (top < contentY)
                contentY = Math.max(0, top - 6)
            else if (bottom > contentY + height)
                contentY = Math.min(contentHeight - height, bottom - height + 6)
        }

        Column {
            id: contentColumn
            width: viewport.width
            height: childrenRect.height
            spacing: 9

            Row {
                width: parent.width
                spacing: 9

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 7
                    height: 7
                    radius: 4
                    color: root.responseBusy ? Theme.warning : Theme.accentStrong
                }

                Text {
                    width: parent.width - statusText.width - 25
                    text: root.backend.aiRequestTitle
                    textFormat: Text.PlainText
                    color: Theme.foreground
                    elide: Text.ElideRight
                    font.family: Theme.uiFont
                    font.pixelSize: 13
                    font.weight: Font.DemiBold
                }

                Text {
                    id: statusText
                    text: root.responseBusy ? "RESPONDING" : "ACTION REQUIRED"
                    color: root.responseBusy ? Theme.warning : Theme.accentStrong
                    font.family: Theme.monoFont
                    font.pixelSize: 9
                    font.letterSpacing: 0.6
                }
            }

            Text {
                width: parent.width
                text: root.backend.aiRequestDescription
                textFormat: Text.PlainText
                visible: text.length > 0
                color: Theme.muted
                wrapMode: Text.WordWrap
                font.family: root.approvalRequest ? Theme.monoFont : Theme.uiFont
                font.pixelSize: 11
                lineHeight: 1.25
            }

            Text {
                width: parent.width
                text: root.backend.aiRequestReason
                textFormat: Text.PlainText
                visible: text.length > 0
                color: Theme.faint
                wrapMode: Text.WordWrap
                font.family: Theme.uiFont
                font.pixelSize: 10
            }

            Repeater {
                id: questionRepeater
                model: root.questions

                delegate: Column {
                    id: questionColumn

                    required property var modelData
                    readonly property bool answerVisible: !modelData.options
                        || modelData.options.length === 0 || modelData.isOther
                    width: contentColumn.width
                    spacing: 6

                    Text {
                        width: parent.width
                        text: questionColumn.modelData.header
                            ? questionColumn.modelData.header.toUpperCase()
                            : "QUESTION"
                        textFormat: Text.PlainText
                        color: Theme.faint
                        font.family: Theme.monoFont
                        font.pixelSize: 9
                        font.letterSpacing: 0.5
                    }

                    Text {
                        width: parent.width
                        text: questionColumn.modelData.question || ""
                        textFormat: Text.PlainText
                        color: Theme.foreground
                        wrapMode: Text.WordWrap
                        font.family: Theme.uiFont
                        font.pixelSize: 11
                    }

                    Flow {
                        id: optionsFlow
                        width: parent.width
                        height: childrenRect.height
                        spacing: 6
                        visible: questionColumn.modelData.options
                            && questionColumn.modelData.options.length > 0

                        Repeater {
                            id: optionRepeater
                            model: questionColumn.modelData.options || []

                            delegate: Item {
                                id: optionItem

                                required property var modelData
                                readonly property bool selected: root.answerFor(
                                    questionColumn.modelData.id) === modelData.label
                                width: Math.min(optionLabel.implicitWidth + 20,
                                    optionsFlow.width)
                                height: Math.max(28, optionLabel.implicitHeight + 12)
                                activeFocusOnTab: root.canRespond
                                Accessible.role: Accessible.RadioButton
                                Accessible.name: modelData.label
                                Accessible.description: modelData.description || ""
                                Accessible.checkable: true
                                Accessible.checked: selected
                                Accessible.onPressAction: root.setAnswer(
                                    questionColumn.modelData.id, optionItem.modelData.label)

                                Rectangle {
                                    anchors.fill: parent
                                    radius: 5
                                    color: optionItem.selected ? Theme.accentMuted
                                        : (optionPointer.containsMouse ? Theme.hover : Theme.input)
                                    border.width: 1
                                    border.color: optionItem.selected || optionItem.activeFocus
                                        ? Theme.accentStrong : Theme.border
                                }

                                Text {
                                    id: optionLabel
                                    anchors.centerIn: parent
                                    width: parent.width - 20
                                    text: optionItem.modelData.label
                                    textFormat: Text.PlainText
                                    color: Theme.foreground
                                    horizontalAlignment: Text.AlignHCenter
                                    wrapMode: Text.Wrap
                                    maximumLineCount: 3
                                    elide: Text.ElideRight
                                    font.family: Theme.uiFont
                                    font.pixelSize: 10
                                }

                                onActiveFocusChanged: {
                                    if (activeFocus)
                                        viewport.reveal(optionItem)
                                }

                                MouseArea {
                                    id: optionPointer
                                    anchors.fill: parent
                                    enabled: root.canRespond
                                    hoverEnabled: true
                                    cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                                    onClicked: root.setAnswer(
                                        questionColumn.modelData.id,
                                        optionItem.modelData.label
                                    )
                                }

                                Keys.onReturnPressed: event => {
                                    root.setAnswer(questionColumn.modelData.id, optionItem.modelData.label)
                                    event.accepted = true
                                }
                                Keys.onSpacePressed: event => {
                                    root.setAnswer(questionColumn.modelData.id, optionItem.modelData.label)
                                    event.accepted = true
                                }
                            }
                        }
                    }

                    Text {
                        width: parent.width
                        text: root.selectedOptionDescription(questionColumn.modelData)
                        textFormat: Text.PlainText
                        visible: text.length > 0
                        color: Theme.faint
                        wrapMode: Text.WordWrap
                        font.family: Theme.uiFont
                        font.pixelSize: 10
                    }

                    Rectangle {
                        id: answerContainer
                        width: parent.width
                        height: 34
                        radius: 5
                        visible: questionColumn.answerVisible
                        color: Theme.input
                        border.width: answerInput.activeFocus ? 1 : 0
                        border.color: Theme.accentStrong

                        TextInput {
                            id: answerInput
                            objectName: questionColumn.modelData.isSecret
                                ? "aiSecretAnswerInput"
                                : "aiAnswerInput-" + questionColumn.modelData.id
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            anchors.rightMargin: 10
                            enabled: root.canRespond
                            activeFocusOnTab: enabled && questionColumn.answerVisible
                            verticalAlignment: TextInput.AlignVCenter
                            color: Theme.foreground
                            selectionColor: Theme.accent
                            selectedTextColor: Theme.foreground
                            echoMode: questionColumn.modelData.isSecret
                                ? TextInput.Password : TextInput.Normal
                            font.family: Theme.uiFont
                            font.pixelSize: 11
                            Accessible.role: Accessible.EditableText
                            Accessible.name: questionColumn.modelData.question
                                || questionColumn.modelData.header || "Answer"
                            Accessible.description: questionColumn.modelData.isSecret
                                ? "Secret answer. Characters are hidden." : ""
                            onActiveFocusChanged: {
                                if (activeFocus)
                                    viewport.reveal(answerInput)
                            }
                            onTextEdited: root.setAnswer(questionColumn.modelData.id, text)
                        }

                        Binding {
                            target: answerInput
                            property: "text"
                            value: root.answerFor(questionColumn.modelData.id)
                        }
                    }
                }
            }

            Text {
                width: parent.width
                visible: root.parseError || !root.backend.aiRequestAnswerable
                text: root.parseError
                    ? "This request could not be decoded safely."
                    : "This request cannot be answered safely in this version of Hunk."
                color: Theme.negative
                font.family: Theme.uiFont
                font.pixelSize: 10
                wrapMode: Text.WordWrap
            }

            Row {
                spacing: 8

                ActionButton {
                    id: acceptAction
                    objectName: "aiApprovalAcceptButton"
                    label: root.responseBusy ? "Responding" : "Accept"
                    primary: true
                    compact: true
                    visible: root.approvalRequest
                    enabled: root.canRespond
                    onClicked: root.resolveApproval(true)
                    onActiveFocusChanged: {
                        if (activeFocus)
                            viewport.reveal(acceptAction)
                    }
                }

                ActionButton {
                    id: declineAction
                    objectName: "aiApprovalDeclineButton"
                    label: "Decline"
                    danger: true
                    compact: true
                    visible: root.approvalRequest
                    enabled: root.canRespond
                    onClicked: root.resolveApproval(false)
                    onActiveFocusChanged: {
                        if (activeFocus)
                            viewport.reveal(declineAction)
                    }
                }

                ActionButton {
                    id: submitAction
                    objectName: "aiInputSubmitButton"
                    label: root.responseBusy ? "Submitting" : "Continue"
                    primary: true
                    compact: true
                    visible: root.inputRequest
                    enabled: root.canRespond
                    onClicked: root.submitInput()
                    onActiveFocusChanged: {
                        if (activeFocus)
                            viewport.reveal(submitAction)
                    }
                }

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    visible: root.inputRequest && root.questions.some(
                        question => question.isSecret)
                    text: "SECRET ANSWERS ARE NOT SAVED"
                    color: Theme.faint
                    font.family: Theme.monoFont
                    font.pixelSize: 8
                    font.letterSpacing: 0.4
                }
            }
        }
    }

    Rectangle {
        anchors.right: parent.right
        anchors.rightMargin: 6
        y: viewport.y + viewport.visibleArea.yPosition * viewport.height
        width: 3
        height: Math.max(24, viewport.visibleArea.heightRatio * viewport.height)
        radius: 2
        visible: root.hasRequest && viewport.contentHeight > viewport.height
        color: Theme.borderStrong
    }

    Connections {
        target: root.backend

        function onAiStateChanged() {
            root.syncBackendState()
        }
    }

    Component.onCompleted: syncBackendState()
}
