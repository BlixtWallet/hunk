pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property QtObject backend
    property Component browserSurfaceComponent: null
    readonly property var workspaceIds: ["diff", "git", "ai"]
    readonly property int workspaceCount: workspaceIds.length
    readonly property string activeWorkspace: backend.activeWorkspace
    readonly property var sidebarItem: sidebarLoader.item
    readonly property var workspaceItem: workspaceLoader.item
    property var aiDraftStore: ({})
    property var aiRequestAnswerStore: ({})
    property string aiDraftWorkspaceRoot: ""
    property real terminalDrawerHeight: 330
    readonly property real minimumWorkspaceHeight: activeWorkspace === "ai" && backend.browser.open ? 300 : 160
    property bool terminalWasOpen: false
    property Item terminalPreviousFocusItem: null
    readonly property int observedTerminalFocusRevision: backend.terminalFocusRevision
    readonly property bool browserFrameRequested: {
        const hostWindow = root.Window.window
        return root.activeWorkspace === "ai"
            && root.backend.browser.open
            && hostWindow !== null
            && hostWindow.visible
            && hostWindow.visibility !== Window.Minimized
    }

    function activateWorkspace(workspace) {
        backend.select_workspace(workspace)
    }

    function syncAiWorkspaceState() {
        if (aiDraftWorkspaceRoot !== backend.aiWorkspaceRoot) {
            aiDraftWorkspaceRoot = backend.aiWorkspaceRoot
            aiDraftStore = ({})
            aiRequestAnswerStore = ({})
        }
        const retainedAnswers = {}
        for (const requestId in aiRequestAnswerStore) {
            if (backend.ai_request_pending(requestId))
                retainedAnswers[requestId] = aiRequestAnswerStore[requestId]
        }
        aiRequestAnswerStore = retainedAnswers
    }

    function setTerminalOpen(open) {
        if (open && !backend.terminalOpen)
            captureTerminalFocus()
        backend.set_terminal_open(open)
        syncTerminalFocus()
    }

    function captureTerminalFocus() {
        const hostWindow = root.Window.window
        terminalPreviousFocusItem = hostWindow === null
            ? null : hostWindow.activeFocusItem
    }

    function syncTerminalFocus() {
        const open = backend.terminalOpen
        if (open === terminalWasOpen)
            return
        if (open && terminalPreviousFocusItem === null)
            captureTerminalFocus()
        terminalWasOpen = open
        if (open) {
            Qt.callLater(() => {
                if (terminalDrawerLoader.item !== null)
                    terminalDrawerLoader.item.focusTerminal()
            })
        } else {
            Qt.callLater(() => {
                if (terminalPreviousFocusItem !== null
                        && terminalPreviousFocusItem.visible
                        && terminalPreviousFocusItem.enabled) {
                    terminalPreviousFocusItem.forceActiveFocus()
                } else if (workspaceLoader.item !== null) {
                    workspaceLoader.item.forceActiveFocus()
                }
                terminalPreviousFocusItem = null
            })
        }
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.canvas
    }

    Rectangle {
        id: header
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Theme.headerHeight
        color: Theme.chrome

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.border
        }

        Row {
            anchors.left: parent.left
            anchors.leftMargin: 16
            height: parent.height
            spacing: 8

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: "HUNK"
                color: Theme.foreground
                font.family: Theme.uiFont
                font.pixelSize: 13
                font.weight: Font.Bold
                font.letterSpacing: 1.2
            }

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 1
                height: 20
                color: Theme.border
            }

            Repeater {
                model: [
                    { label: "Diff", workspace: "diff" },
                    { label: "Git", workspace: "git" },
                    { label: "AI", workspace: "ai" }
                ]

                delegate: NavigationButton {
                    required property var modelData
                    label: modelData.label
                    workspace: modelData.workspace
                    selected: root.activeWorkspace === workspace
                    onActivated: workspace => root.activateWorkspace(workspace)
                }
            }
        }

        Row {
            anchors.right: parent.right
            anchors.rightMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            spacing: 8

            ActionButton {
                anchors.verticalCenter: parent.verticalCenter
                label: root.backend.terminalOpen ? qsTr("Terminal ·") : qsTr("Terminal")
                accessibleName: root.backend.terminalOpen
                    ? qsTr("Close terminal") : qsTr("Open terminal")
                compact: true
                onClicked: root.setTerminalOpen(!root.backend.terminalOpen)
            }

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 7
                height: 7
                radius: 4
                color: root.backend.ready ? Theme.positive : Theme.muted
            }

            Text {
                width: Math.min(260, implicitWidth)
                text: root.backend.statusMessage
                color: Theme.muted
                elide: Text.ElideRight
                font.family: Theme.uiFont
                font.pixelSize: 11
            }
        }
    }

    Item {
        id: sidebar
        anchors.left: parent.left
        anchors.top: header.bottom
        anchors.bottom: parent.bottom
        width: Theme.sidebarWidth

        Loader {
            id: sidebarLoader
            anchors.fill: parent
            sourceComponent: root.activeWorkspace === "git" ? gitSidebarComponent
                : (root.activeWorkspace === "diff" ? diffSidebarComponent : aiSidebarComponent)
        }

        Rectangle {
            anchors.right: parent.right
            width: 1
            height: parent.height
            color: Theme.border
        }
    }

    Loader {
        id: workspaceLoader
        objectName: "workspaceLoader"
        anchors.left: sidebar.right
        anchors.right: parent.right
        anchors.top: header.bottom
        anchors.bottom: terminalDrawerLoader.top
        sourceComponent: root.activeWorkspace === "git" ? gitWorkspaceComponent
            : (root.activeWorkspace === "diff" ? diffWorkspaceComponent : aiWorkspaceComponent)
    }

    Loader {
        id: terminalDrawerLoader
        objectName: "terminalDrawer"
        anchors.left: sidebar.right
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: active
            ? Math.min(root.terminalDrawerHeight,
                Math.max(180, root.height - Theme.headerHeight - root.minimumWorkspaceHeight)) : 0
        active: root.backend.terminalOpen
        visible: active
        sourceComponent: terminalDrawerComponent
        onLoaded: Qt.callLater(() => {
            if (item !== null)
                item.focusTerminal()
        })
    }

    Component {
        id: terminalDrawerComponent

        TerminalDrawer {
            backend: root.backend
            onResizeRequested: height => {
                root.terminalDrawerHeight = Math.max(180,
                    Math.min(height, root.height - Theme.headerHeight - root.minimumWorkspaceHeight))
            }
            onCloseRequested: root.setTerminalOpen(false)
        }
    }

    Component {
        id: diffSidebarComponent

        DiffSidebar {
            backend: root.backend
        }
    }

    Component {
        id: diffWorkspaceComponent

        DiffWorkspace {
            objectName: "diffWorkspace"
            backend: root.backend
        }
    }

    Component {
        id: gitSidebarComponent

        GitSidebar {
            backend: root.backend
        }
    }

    Component {
        id: gitWorkspaceComponent

        GitWorkspace {
            objectName: "gitWorkspace"
            backend: root.backend
        }
    }

    Component {
        id: aiSidebarComponent

        AiSidebar {
            backend: root.backend
        }
    }

    Component {
        id: aiWorkspaceComponent

        AiWorkspace {
            objectName: "aiWorkspace"
            backend: root.backend
            draftStore: root.aiDraftStore
            requestAnswerStore: root.aiRequestAnswerStore
            browserSurfaceComponent: root.browserSurfaceComponent
        }
    }

    Timer {
        interval: 16
        repeat: true
        running: root.backend.browser.pumpActive
        onTriggered: root.backend.browser.pump(root.browserFrameRequested)
    }

    ConfirmationDialog {
        anchors.fill: parent
        visible: root.backend.browser.approvalPending
        title: root.backend.browser.approvalKind
        message: root.backend.browser.approvalSummary + "\n\nCodex requested this sensitive browser action."
        confirmLabel: qsTr("Allow once")
        onAccepted: root.backend.browser.resolve_approval(true)
        onRejected: root.backend.browser.resolve_approval(false)
        onFocusRestorationFailed: {
            if (root.activeWorkspace === "ai"
                    && root.backend.browser.open
                    && workspaceLoader.item !== null) {
                workspaceLoader.item.focusBrowserSurface()
            }
        }
    }

    Connections {
        target: root.backend

        function onAiStateChanged() {
            root.syncAiWorkspaceState()
        }

        function onTerminalStateChanged() {
            root.syncTerminalFocus()
        }

        function onTerminalFocusChanged() {
            root.syncTerminalFocus()
        }
    }

    Shortcut {
        sequence: Qt.platform.os === "osx" ? "Meta+J" : "Ctrl+J"
        autoRepeat: false
        onActivated: root.setTerminalOpen(!root.backend.terminalOpen)
    }

    Component.onCompleted: {
        syncAiWorkspaceState()
        terminalWasOpen = backend.terminalOpen
    }

    onObservedTerminalFocusRevisionChanged: {
        if (backend.terminalOpen)
            Qt.callLater(() => {
                if (terminalDrawerLoader.item !== null)
                    terminalDrawerLoader.item.focusTerminal()
            })
    }
}
