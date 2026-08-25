pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property var backend
    property bool followTail: true
    property string visibleThreadId: backend.aiActiveThreadId
    readonly property alias timelineListView: timeline
    readonly property bool errorStateVisible: backend.aiError.length > 0 && !backend.aiReady
    readonly property bool loadingStateVisible: backend.aiLoading && !backend.aiReady
        && !errorStateVisible
    readonly property bool timelineStateVisible: backend.aiActiveThreadId.length > 0
        && timeline.count > 0 && !errorStateVisible
    readonly property bool authenticationStateVisible: backend.aiRequiresAuthentication
        && !timelineStateVisible && !errorStateVisible && !loadingStateVisible
    readonly property bool emptyStateVisible: !timelineStateVisible && !errorStateVisible
        && !loadingStateVisible && !authenticationStateVisible

    function stateTitle() {
        if (root.errorStateVisible)
            return "Codex is unavailable"
        if (root.loadingStateVisible)
            return "Connecting to Codex…"
        if (root.authenticationStateVisible)
            return "OpenAI sign-in required"
        if (root.backend.aiActiveThreadId.length === 0)
            return root.backend.aiThreadCount > 0 ? "Select a thread" : "No Codex threads"
        return "No messages yet"
    }

    function stateDescription() {
        if (root.errorStateVisible)
            return root.backend.aiError
        if (root.loadingStateVisible)
            return "Starting the repository-scoped Codex worker and loading threads."
        if (root.authenticationStateVisible)
            return "Complete authentication through the Codex runtime before starting a turn."
        if (root.backend.aiActiveThreadId.length === 0)
            return root.backend.aiThreadCount > 0
                ? "Choose a thread from the catalog to load its conversation."
                : "Create a thread from the sidebar to begin."
        return "This thread does not contain a visible turn yet."
    }

    onVisibleThreadIdChanged: {
        followTail = true
        Qt.callLater(() => timeline.positionViewAtEnd())
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.canvas
    }

    Item {
        id: workspaceHeader
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 66

        Column {
            anchors.left: parent.left
            anchors.right: headerActions.left
            anchors.leftMargin: 20
            anchors.rightMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            spacing: 3

            Text {
                width: parent.width
                text: root.backend.aiActiveThreadTitle || "Codex"
                textFormat: Text.PlainText
                color: Theme.foreground
                elide: Text.ElideRight
                font.family: Theme.uiFont
                font.pixelSize: 17
                font.weight: Font.DemiBold
            }

            Text {
                width: parent.width
                text: root.backend.aiActiveThreadCwd || root.backend.aiWorkspaceRoot
                textFormat: Text.PlainText
                color: Theme.faint
                elide: Text.ElideMiddle
                font.family: Theme.monoFont
                font.pixelSize: 10
            }
        }

        Row {
            id: headerActions
            anchors.right: parent.right
            anchors.rightMargin: 20
            anchors.verticalCenter: parent.verticalCenter
            spacing: 8

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: "READ ONLY"
                color: Theme.faint
                font.family: Theme.monoFont
                font.pixelSize: 9
                font.letterSpacing: 0.7
            }

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 7
                height: 7
                radius: 4
                color: root.backend.aiConnectionState === "ready" ? Theme.positive
                    : (root.backend.aiConnectionState === "failed" ? Theme.negative : Theme.warning)
            }

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: root.backend.aiConnectionState.toUpperCase()
                textFormat: Text.PlainText
                color: Theme.muted
                font.family: Theme.monoFont
                font.pixelSize: 9
                font.letterSpacing: 0.5
            }
        }

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.border
        }
    }

    Rectangle {
        id: statusBanner
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: workspaceHeader.bottom
        height: visible ? 34 : 0
        visible: root.backend.aiError.length > 0
            || root.backend.aiRequiresAuthentication
            || root.backend.aiLoading
            || root.backend.aiStatusMessage.length > 0
        color: root.backend.aiError.length > 0 ? Theme.negativeMuted
            : (root.backend.aiRequiresAuthentication ? Theme.accentMuted : Theme.raised)

        Text {
            anchors.left: parent.left
            anchors.right: statusLabel.left
            anchors.leftMargin: 20
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            text: root.backend.aiError.length > 0 ? root.backend.aiError
                : (root.backend.aiRequiresAuthentication ? "OpenAI authentication is required."
                    : (root.backend.aiLoading ? "Loading Codex threads…"
                        : root.backend.aiStatusMessage))
            textFormat: Text.PlainText
            color: root.backend.aiError.length > 0 ? Theme.negative : Theme.muted
            elide: Text.ElideRight
            font.family: Theme.uiFont
            font.pixelSize: 11
        }

        Text {
            id: statusLabel
            anchors.right: parent.right
            anchors.rightMargin: 20
            anchors.verticalCenter: parent.verticalCenter
            text: root.backend.aiError.length > 0 ? "ERROR"
                : (root.backend.aiRequiresAuthentication ? "SIGN IN"
                    : (root.backend.aiLoading ? "LOADING" : "STATUS"))
            color: Theme.faint
            font.family: Theme.monoFont
            font.pixelSize: 9
            font.letterSpacing: 0.6
        }
    }

    Item {
        id: timelinePane
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: statusBanner.bottom
        anchors.bottom: parent.bottom

        Rectangle {
            id: historyNotice
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: visible ? 28 : 0
            visible: root.timelineStateVisible
                && (root.backend.aiTimelineHiddenTurnCount > 0
                    || root.backend.aiTimelineHiddenRowCount > 0)
            color: Theme.chrome

            Text {
                anchors.centerIn: parent
                text: {
                    if (root.backend.aiTimelineHiddenTurnCount > 0)
                        return "Showing the latest " + root.backend.aiTimelineVisibleTurnCount
                            + " of " + root.backend.aiTimelineTotalTurnCount + " turns"
                    return root.backend.aiTimelineHiddenRowCount + " earlier timeline rows hidden"
                }
                color: Theme.faint
                font.family: Theme.monoFont
                font.pixelSize: 9
            }

            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: 1
                color: Theme.border
            }
        }

        ListView {
            id: timeline
            objectName: "aiTimelineList"
            anchors.top: historyNotice.bottom
            anchors.bottom: parent.bottom
            anchors.horizontalCenter: parent.horizontalCenter
            width: Math.min(780, parent.width - 48)
            visible: root.timelineStateVisible
            clip: true
            spacing: 2
            model: root.backend.aiTimeline
            boundsBehavior: Flickable.StopAtBounds
            reuseItems: true
            cacheBuffer: Math.max(height, 640)
            topMargin: 18
            bottomMargin: 24

            delegate: AiTimelineRow {
                width: timeline.width
            }

            onCountChanged: {
                if (root.followTail)
                    Qt.callLater(() => timeline.positionViewAtEnd())
            }
            onContentHeightChanged: {
                if (root.followTail)
                    Qt.callLater(() => timeline.positionViewAtEnd())
            }
            onMovementStarted: root.followTail = false
            onMovementEnded: root.followTail = timeline.atYEnd
        }

        Column {
            anchors.centerIn: parent
            width: Math.min(460, parent.width - 48)
            spacing: 8
            visible: root.errorStateVisible || root.loadingStateVisible
                || root.authenticationStateVisible || root.emptyStateVisible

            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: root.stateTitle()
                color: root.errorStateVisible ? Theme.negative : Theme.foreground
                font.family: Theme.uiFont
                font.pixelSize: 17
                font.weight: Font.DemiBold
            }

            Text {
                width: parent.width
                text: root.stateDescription()
                textFormat: Text.PlainText
                color: Theme.muted
                horizontalAlignment: Text.AlignHCenter
                lineHeight: 1.3
                wrapMode: Text.WordWrap
                font.family: Theme.uiFont
                font.pixelSize: 12
            }
        }
    }
}
