pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property var backend
    property string pendingArchiveId: ""
    property string pendingArchiveTitle: ""
    property bool archiveConfirmationVisible: false
    readonly property alias threadListView: threadList
    readonly property alias archiveDialog: archiveConfirmation
    readonly property bool loadingStateVisible: backend.aiLoading && !backend.aiReady
    readonly property bool emptyStateVisible: threadList.count === 0
        && backend.aiReady && !backend.aiLoading

    function selectThread(threadId) {
        if (threadId.length > 0 && threadId !== backend.aiActiveThreadId)
            backend.select_ai_thread(threadId)
    }

    function refreshThreads() {
        backend.refresh_ai_threads()
    }

    function createThread() {
        backend.create_ai_thread()
    }

    function requestArchive(threadId, title) {
        pendingArchiveId = threadId
        pendingArchiveTitle = title
        archiveConfirmationVisible = threadId.length > 0
    }

    function cancelArchive() {
        pendingArchiveId = ""
        pendingArchiveTitle = ""
        archiveConfirmationVisible = false
    }

    function confirmArchive() {
        const threadId = pendingArchiveId
        cancelArchive()
        if (threadId.length > 0)
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
        height: 108

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
                label: "Refresh"
                compact: true
                enabled: !root.backend.aiLoading
                onClicked: root.refreshThreads()
            }

            ActionButton {
                label: "New"
                compact: true
                primary: true
                enabled: !root.backend.aiLoading && !root.backend.aiRequiresAuthentication
                onClicked: root.createThread()
            }
        }

        Text {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.leftMargin: 16
            anchors.rightMargin: 16
            anchors.topMargin: 43
            text: root.backend.gitRepositoryName || "Repository"
            textFormat: Text.PlainText
            color: Theme.foreground
            elide: Text.ElideRight
            font.family: Theme.uiFont
            font.pixelSize: 15
            font.weight: Font.DemiBold
        }

        Row {
            anchors.left: parent.left
            anchors.leftMargin: 16
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 10
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
        }

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.border
        }
    }

    ListView {
        id: threadList
        objectName: "aiThreadList"
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: catalogHeader.bottom
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
            required property double created_at
            required property double updated_at

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
                anchors.right: archiveAction.left
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
                    text: threadRow.running ? "RUNNING"
                        : threadRow.workspace_label + "  ·  " + threadRow.status.toUpperCase()
                    textFormat: Text.PlainText
                    color: threadRow.running ? Theme.positive : Theme.faint
                    elide: Text.ElideRight
                    font.family: Theme.monoFont
                    font.pixelSize: 9
                    font.letterSpacing: 0.4
                }
            }

            ActionButton {
                id: archiveAction
                anchors.right: parent.right
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                label: "Archive"
                compact: true
                visible: threadRow.active || rowHover.hovered
                enabled: !root.backend.aiLoading
                z: 2
                onClicked: root.requestArchive(threadRow.thread_id, threadRow.title)
            }

            MouseArea {
                id: threadPointer
                anchors.fill: parent
                enabled: !threadRow.active && !root.backend.aiLoading
                cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                onClicked: root.selectThread(threadRow.thread_id)
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
