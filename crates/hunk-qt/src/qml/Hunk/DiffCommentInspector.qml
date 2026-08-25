pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property var backend
    readonly property alias listView: commentList
    readonly property bool emptyStateVisible: emptyState.visible
    signal closeRequested
    signal copyRequested(string text)

    function copyAllOpen() {
        const text = backend.diff_all_open_comment_bundles()
        if (text.length > 0)
            copyRequested(text)
    }

    function copyComment(text) {
        if (text.length > 0)
            copyRequested(text)
    }

    function jumpToComment(commentId) {
        backend.jump_to_diff_comment(commentId)
    }

    function setCommentStatus(commentId, status) {
        backend.set_diff_comment_status(commentId, status)
    }

    function deleteComment(commentId) {
        backend.delete_diff_comment(commentId)
    }

    function toggleNonOpen() {
        backend.set_diff_comments_show_non_open(!backend.diffCommentsShowNonOpen)
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.chrome
    }

    Rectangle {
        anchors.left: parent.left
        width: 1
        height: parent.height
        color: Theme.borderStrong
    }

    Item {
        id: header
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 54

        Column {
            anchors.left: parent.left
            anchors.leftMargin: 14
            anchors.right: headerActions.left
            anchors.rightMargin: 10
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2

            Text {
                width: parent.width
                text: "COMMENTS"
                color: Theme.foreground
                elide: Text.ElideRight
                font.family: Theme.uiFont
                font.pixelSize: 11
                font.weight: Font.DemiBold
                font.letterSpacing: 0.8
            }

            Text {
                width: parent.width
                text: root.backend.diffCommentsOpenCount + " open · "
                    + root.backend.diffCommentsStaleCount + " stale · "
                    + root.backend.diffCommentsResolvedCount + " resolved"
                color: Theme.faint
                elide: Text.ElideRight
                font.family: Theme.monoFont
                font.pixelSize: 8
            }
        }

        Row {
            id: headerActions
            anchors.right: parent.right
            anchors.rightMargin: 10
            anchors.verticalCenter: parent.verticalCenter
            spacing: 5

            ActionButton {
                label: "Copy all"
                compact: true
                enabled: root.backend.diffCommentsOpenCount > 0
                onClicked: root.copyAllOpen()
            }

            ActionButton {
                label: "×"
                compact: true
                onClicked: root.closeRequested()
            }
        }

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.border
        }
    }

    Item {
        id: controls
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: header.bottom
        height: 38

        ActionButton {
            anchors.left: parent.left
            anchors.leftMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            label: root.backend.diffCommentsShowNonOpen ? "Open only" : "Show history"
            compact: true
            enabled: !root.backend.diffCommentsLoading && !root.backend.diffCommentsBusy
            onClicked: root.toggleNonOpen()
        }

        Text {
            anchors.right: parent.right
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            visible: root.backend.diffCommentsLoading || root.backend.diffCommentsBusy
            text: root.backend.diffCommentsLoading ? "LOADING" : "SAVING"
            color: Theme.faint
            font.family: Theme.monoFont
            font.pixelSize: 8
            font.letterSpacing: 0.7
        }

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.border
        }
    }

    Item {
        id: messageBar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: controls.bottom
        height: message.visible ? 34 : 0
        clip: true

        Text {
            id: message
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.leftMargin: 12
            anchors.rightMargin: 12
            visible: root.backend.diffCommentsError.length > 0
                || root.backend.diffCommentsStatusMessage.length > 0
            text: root.backend.diffCommentsError.length > 0
                ? root.backend.diffCommentsError : root.backend.diffCommentsStatusMessage
            color: root.backend.diffCommentsError.length > 0 ? Theme.negative : Theme.muted
            elide: Text.ElideRight
            font.family: Theme.uiFont
            font.pixelSize: 9
        }

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: message.visible ? 1 : 0
            color: Theme.border
        }
    }

    ListView {
        id: commentList
        objectName: "diffCommentList"
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: messageBar.bottom
        anchors.bottom: parent.bottom
        model: root.backend.diffComments
        clip: true
        reuseItems: true
        cacheBuffer: 640
        boundsBehavior: Flickable.StopAtBounds

        delegate: Item {
            id: commentRow

            required property string comment_id
            required property string status
            required property string file_path
            required property string line_hint
            required property string comment_text
            required property string clipboard_text
            required property int row
            required property bool can_jump

            width: commentList.width
            height: 126

            Column {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: 12
                spacing: 5

                Row {
                    width: parent.width
                    spacing: 7

                    Text {
                        id: statusLabel
                        text: commentRow.status.toUpperCase()
                        color: commentRow.status === "open" ? Theme.accentStrong
                            : (commentRow.status === "stale" ? Theme.warning : Theme.muted)
                        font.family: Theme.monoFont
                        font.pixelSize: 8
                        font.weight: Font.Bold
                    }

                    Text {
                        width: parent.width - statusLabel.implicitWidth - 7
                        text: commentRow.file_path + " · " + commentRow.line_hint
                        color: Theme.faint
                        elide: Text.ElideMiddle
                        font.family: Theme.monoFont
                        font.pixelSize: 8
                    }
                }

                Text {
                    width: parent.width
                    height: 42
                    text: commentRow.comment_text
                    color: Theme.foreground
                    elide: Text.ElideRight
                    maximumLineCount: 3
                    wrapMode: Text.WordWrap
                    font.family: Theme.uiFont
                    font.pixelSize: 11
                }

                Row {
                    spacing: 5

                    ActionButton {
                        label: "Go"
                        compact: true
                        enabled: commentRow.can_jump
                            && !root.backend.diffCommentsLoading
                            && !root.backend.diffCommentsBusy
                        onClicked: root.jumpToComment(commentRow.comment_id)
                    }

                    ActionButton {
                        label: "Copy"
                        compact: true
                        onClicked: root.copyComment(commentRow.clipboard_text)
                    }

                    ActionButton {
                        label: commentRow.status === "open" ? "Resolve" : "Reopen"
                        compact: true
                        enabled: !root.backend.diffCommentsLoading
                            && !root.backend.diffCommentsBusy
                        onClicked: root.setCommentStatus(
                            commentRow.comment_id,
                            commentRow.status === "open" ? "resolved" : "open"
                        )
                    }

                    ActionButton {
                        label: "Delete"
                        compact: true
                        danger: true
                        enabled: !root.backend.diffCommentsLoading
                            && !root.backend.diffCommentsBusy
                        onClicked: root.deleteComment(commentRow.comment_id)
                    }
                }
            }

            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: 1
                color: Theme.border
            }
        }
    }

    Text {
        id: emptyState
        anchors.centerIn: commentList
        width: parent.width - 32
        visible: root.backend.diffCommentsReady
            && !root.backend.diffCommentsLoading
            && commentList.count === 0
        text: root.backend.diffCommentsShowNonOpen
            ? "No comments for this branch"
            : "No open comments"
        color: Theme.muted
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
        font.family: Theme.uiFont
        font.pixelSize: 11
    }
}
