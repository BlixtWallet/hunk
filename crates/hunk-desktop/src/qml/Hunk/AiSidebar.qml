pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Dialogs

Item {
    id: root

    required property var backend
    property string pendingArchiveId: ""
    property string pendingArchiveTitle: ""
    property bool archiveConfirmationVisible: false
    property Item repositoryPreviousFocusItem: null
    readonly property var projectCatalog: {
        try {
            return JSON.parse(backend.aiProjectCatalogJson)
        } catch (error) {
            return []
        }
    }
    readonly property alias projectListView: projectList
    readonly property alias threadListView: threadList
    readonly property alias archiveDialog: archiveConfirmation
    readonly property alias repositoryDialog: repositoryDialog
    readonly property bool loadingStateVisible: backend.aiLoading && !backend.aiReady
    readonly property bool emptyStateVisible: threadList.count === 0
        && backend.aiReady && !backend.aiLoading
    readonly property bool commandPending: backend.aiThreadActionPending
        || backend.aiPromptPending
        || backend.aiInterruptPending || backend.aiRequestId.length > 0
        || backend.aiRequestResolving

    onCommandPendingChanged: {
        if (commandPending && archiveConfirmationVisible)
            cancelArchive()
    }

    function selectThread(threadId) {
        if (threadId.length > 0 && threadId !== backend.aiActiveThreadId)
            backend.select_ai_thread(threadId)
    }

    function selectProject(projectPath) {
        if (projectPath.length > 0 && projectPath !== backend.gitRoot
                && !backend.gitLoading && !backend.aiTurnRunning && !commandPending)
            backend.select_git_root(projectPath)
    }

    function refreshThreads() {
        backend.refresh_ai_threads()
    }

    function createThread() {
        backend.create_ai_thread()
    }

    function openRepository() {
        const hostWindow = root.Window.window
        repositoryPreviousFocusItem = hostWindow === null
            ? null : hostWindow.activeFocusItem
        repositoryDialog.open()
    }

    function restoreRepositoryFocus() {
        const previous = repositoryPreviousFocusItem
        repositoryPreviousFocusItem = null
        Qt.callLater(() => {
            if (previous !== null && previous.visible && previous.enabled)
                previous.forceActiveFocus()
            else
                threadList.forceActiveFocus()
        })
    }

    function toggleBookmark(threadId) {
        if (threadId.length > 0)
            backend.toggle_ai_thread_bookmark(threadId)
    }

    function requestArchive(threadId, title) {
        pendingArchiveId = threadId
        pendingArchiveTitle = title
        archiveConfirmationVisible = threadId.length > 0 && !commandPending
    }

    function cancelArchive() {
        pendingArchiveId = ""
        pendingArchiveTitle = ""
        archiveConfirmationVisible = false
    }

    function confirmArchive() {
        const threadId = pendingArchiveId
        cancelArchive()
        if (threadId.length > 0 && !commandPending)
            backend.archive_ai_thread(threadId)
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.chrome
    }

    Item {
        id: catalogHeader
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 48

        Text {
            anchors.left: parent.left
            anchors.leftMargin: 16
            anchors.top: parent.top
            anchors.topMargin: 15
            text: "CODEX"
            color: Theme.muted
            font.family: Theme.uiFont
            font.pixelSize: 10
            font.weight: Font.DemiBold
            font.letterSpacing: 1.1
        }

        Row {
            anchors.right: parent.right
            anchors.rightMargin: 12
            anchors.top: parent.top
            anchors.topMargin: 10
            spacing: 6

            ActionButton {
                label: "Open"
                compact: true
                enabled: !root.backend.aiLoading && !root.backend.aiTurnRunning
                    && !root.commandPending
                onClicked: root.openRepository()
            }

            ActionButton {
                label: "Refresh"
                compact: true
                enabled: !root.backend.aiLoading && !root.commandPending
                onClicked: root.refreshThreads()
            }

            ActionButton {
                label: "New"
                compact: true
                primary: true
                enabled: !root.backend.aiLoading && !root.backend.aiRequiresAuthentication
                    && !root.commandPending
                onClicked: root.createThread()
            }
        }

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.border
        }
    }

    ListView {
        id: projectList
        objectName: "aiProjectList"
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: catalogHeader.bottom
        height: Math.min(contentHeight, 160)
        clip: true
        model: root.projectCatalog
        boundsBehavior: Flickable.StopAtBounds
        reuseItems: true
        cacheBuffer: 160

        delegate: Rectangle {
            id: projectRow

            required property string project_path
            required property string name
            readonly property bool active: project_path === root.backend.gitRoot

            width: projectList.width
            height: 40
            color: active ? Theme.selected
                : (projectHover.hovered ? Theme.hover : Theme.transparent)
                                activeFocusOnTab: true
            Accessible.role: Accessible.ListItem
            Accessible.name: active ? qsTr("%1, active project").arg(name) : name
            Accessible.onPressAction: {
                if (projectPointer.enabled)
                    root.selectProject(projectRow.project_path)
            }

            Keys.onReturnPressed: event => {
                root.selectProject(projectRow.project_path)
                event.accepted = true
            }
            Keys.onEnterPressed: event => {
                root.selectProject(projectRow.project_path)
                event.accepted = true
            }
            Keys.onSpacePressed: event => {
                root.selectProject(projectRow.project_path)
                event.accepted = true
            }

            HoverHandler {
                id: projectHover
            }

            Rectangle {
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                width: 2
                height: 24
                color: projectRow.active ? Theme.accentStrong : Theme.transparent
            }

            Column {
                anchors.left: parent.left
                anchors.right: projectState.left
                anchors.leftMargin: 16
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2

                Text {
                    width: parent.width
                    text: projectRow.name
                    textFormat: Text.PlainText
                    color: projectRow.active ? Theme.foreground : Theme.muted
                    elide: Text.ElideRight
                    font.family: Theme.uiFont
                    font.pixelSize: 12
                    font.weight: projectRow.active ? Font.DemiBold : Font.Normal
                }

                Text {
                    width: parent.width
                    text: projectRow.project_path
                    textFormat: Text.PlainText
                    color: Theme.faint
                    elide: Text.ElideMiddle
                    font.family: Theme.monoFont
                    font.pixelSize: 8
                }
            }

            Text {
                id: projectState
                anchors.right: parent.right
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                visible: projectRow.active
                text: qsTr("ACTIVE")
                color: Theme.accent
                font.family: Theme.monoFont
                font.pixelSize: 8
                font.letterSpacing: 0.5
            }

            MouseArea {
                id: projectPointer
                anchors.fill: parent
                enabled: !projectRow.active && !root.backend.gitLoading
                    && !root.backend.aiTurnRunning && !root.commandPending
                cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                onClicked: root.selectProject(projectRow.project_path)
            }

            Rectangle {
                anchors.fill: parent
                color: Theme.transparent
                border.width: projectRow.activeFocus ? 1 : 0
                border.color: Theme.accentStrong
            }
        }
    }

    Item {
        id: activeProjectHeader
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: projectList.bottom
        height: 60

        Text {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: 16
            anchors.rightMargin: 16
            anchors.top: parent.top
            anchors.topMargin: 10
            text: root.backend.gitRepositoryName || "Repository"
            textFormat: Text.PlainText
            color: Theme.foreground
            elide: Text.ElideRight
            font.family: Theme.uiFont
            font.pixelSize: 13
            font.weight: Font.DemiBold
        }

        Row {
            anchors.left: parent.left
            anchors.leftMargin: 16
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 9
            spacing: 8

            Text {
                text: root.backend.aiThreadCount + " THREADS"
                color: Theme.faint
                font.family: Theme.monoFont
                font.pixelSize: 9
                font.letterSpacing: 0.6
            }

            Text {
                visible: root.backend.aiRunningThreadCount > 0
                text: root.backend.aiRunningThreadCount + " RUNNING"
                color: Theme.positive
                font.family: Theme.monoFont
                font.pixelSize: 9
                font.letterSpacing: 0.6
            }

            Text {
                visible: root.backend.aiPendingRequestCount > 0
                text: root.backend.aiPendingRequestCount + " ACTION"
                    + (root.backend.aiPendingRequestCount === 1 ? "" : "S")
                color: Theme.warning
                font.family: Theme.monoFont
                font.pixelSize: 9
                font.letterSpacing: 0.6
            }
        }

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.border
        }
    }

    FolderDialog {
        id: repositoryDialog
        title: "Open Git repository"
        onAccepted: root.backend.select_git_root(selectedFolder.toString())
        onVisibleChanged: {
            if (!visible && root.repositoryPreviousFocusItem !== null)
                root.restoreRepositoryFocus()
        }
    }

    ListView {
        id: threadList
        objectName: "aiThreadList"
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: activeProjectHeader.bottom
        anchors.bottom: parent.bottom
        clip: true
        model: root.backend.aiThreads
        boundsBehavior: Flickable.StopAtBounds
        reuseItems: true
        cacheBuffer: Theme.aiThreadRowHeight * 4

        delegate: Rectangle {
            id: threadRow

            required property string thread_id
            required property string title
            required property string cwd
            required property string workspace_label
            required property string status
            required property bool active
            required property bool running
            required property bool attention
            required property bool bookmarked
            required property double created_at
            required property double updated_at
            readonly property alias bookmarkButton: bookmarkAction
            readonly property alias archiveButton: archiveAction

            width: threadList.width
            height: Theme.aiThreadRowHeight
            color: active ? Theme.selected
                : (rowHover.hovered ? Theme.hover : Theme.transparent)

            HoverHandler {
                id: rowHover
            }

            Rectangle {
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                width: 2
                height: 30
                color: threadRow.active ? Theme.accentStrong : Theme.transparent
            }

            Column {
                anchors.left: parent.left
                anchors.right: threadActions.left
                anchors.leftMargin: 16
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4

                Text {
                    width: parent.width
                    text: threadRow.title
                    textFormat: Text.PlainText
                    color: threadRow.active ? Theme.foreground : Theme.muted
                    elide: Text.ElideRight
                    font.family: Theme.uiFont
                    font.pixelSize: 12
                    font.weight: threadRow.active ? Font.DemiBold : Font.Normal
                }

                Text {
                    width: parent.width
                    text: threadRow.attention ? "ACTION REQUIRED"
                        : (threadRow.running ? "RUNNING"
                            : threadRow.workspace_label + "  ·  "
                                + threadRow.status.toUpperCase())
                    textFormat: Text.PlainText
                    color: threadRow.attention ? Theme.warning
                        : (threadRow.running ? Theme.positive : Theme.faint)
                    elide: Text.ElideRight
                    font.family: Theme.monoFont
                    font.pixelSize: 9
                    font.letterSpacing: 0.4
                }
            }

            Row {
                id: threadActions
                anchors.right: parent.right
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4
                visible: threadRow.bookmarked || threadRow.active || rowHover.hovered
                z: 2

                ActionButton {
                    id: bookmarkAction
                    label: threadRow.bookmarked ? "★" : "☆"
                    accessibleName: threadRow.bookmarked
                        ? "Remove bookmark" : "Bookmark thread"
                    compact: true
                    enabled: !root.backend.aiLoading
                    onClicked: root.toggleBookmark(threadRow.thread_id)
                }

                ActionButton {
                    id: archiveAction
                    label: "Archive"
                    compact: true
                    visible: threadRow.active || rowHover.hovered
                    enabled: !threadRow.attention && !root.backend.aiLoading
                        && !root.commandPending
                    onClicked: root.requestArchive(threadRow.thread_id, threadRow.title)
                }
            }

            MouseArea {
                id: threadPointer
                enabled: !threadRow.active && !root.backend.aiLoading && !root.commandPending
                cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                onClicked: root.selectThread(threadRow.thread_id)

                anchors {
                    left: parent.left
                    right: threadActions.left
                    top: parent.top
                    bottom: parent.bottom
                }
            }
        }
    }

    Column {
        anchors.centerIn: threadList
        width: parent.width - 40
        spacing: 7
        visible: root.loadingStateVisible || root.emptyStateVisible

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.loadingStateVisible ? "Connecting to Codex…" : "No threads yet"
            color: Theme.foreground
            font.family: Theme.uiFont
            font.pixelSize: 13
            font.weight: Font.DemiBold
        }

        Text {
            width: parent.width
            text: root.loadingStateVisible ? "Loading this repository's thread catalog."
                : "Create a thread to begin working with Codex."
            color: Theme.muted
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            font.family: Theme.uiFont
            font.pixelSize: 11
        }
    }

    ConfirmationDialog {
        id: archiveConfirmation
        anchors.fill: parent
        visible: root.archiveConfirmationVisible
        title: "Archive thread?"
        message: root.pendingArchiveTitle.length > 0
            ? "Archive ‘" + root.pendingArchiveTitle + "’? It will leave this active catalog."
            : "Archive this Codex thread?"
        confirmLabel: "Archive"
        onAccepted: root.confirmArchive()
        onRejected: root.cancelArchive()
    }
}
