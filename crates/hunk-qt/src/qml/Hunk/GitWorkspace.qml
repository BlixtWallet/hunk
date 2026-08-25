pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property var backend
    property string pendingDiscardPath: ""
    property bool discardConfirmationVisible: false
    property bool forgeTokenDialogVisible: false
    property bool forgeReviewDialogVisible: false
    readonly property alias fileListView: fileList
    readonly property alias commitMessageInput: commitMessage
    readonly property alias forgePanelItem: forgePanel
    readonly property alias forgeTokenDialog: tokenDialog
    readonly property alias forgeReviewDialog: reviewDialog
    readonly property bool loadingStateVisible: backend.gitLoading && !backend.gitReady
    readonly property bool emptyStateVisible: fileList.count === 0
        && backend.gitReady && !backend.gitLoading
    readonly property bool errorStateVisible: backend.gitError.length > 0

    function requestDiscard(path) {
        pendingDiscardPath = path
        discardConfirmationVisible = true
    }

    function cancelDiscard() {
        pendingDiscardPath = ""
        discardConfirmationVisible = false
    }

    function confirmDiscard() {
        const path = pendingDiscardPath
        cancelDiscard()
        if (path.length > 0)
            backend.discard_path(path)
    }

    function submitCommit() {
        const message = commitMessage.text.trim()
        if (message.length === 0 || backend.gitStagedFileCount === 0 || backend.gitBusy)
            return
        backend.commit_staged(message)
        commitMessage.text = ""
    }

    function requestForgeAuthentication() {
        if (backend.forgeAuthMode === "device")
            backend.start_github_device_flow()
        else {
            tokenDialog.prepare()
            forgeTokenDialogVisible = true
        }
    }

    function openForgeReviewDialog() {
        const title = backend.gitLastCommitSubject.length > 0
            ? backend.gitLastCommitSubject
            : backend.gitBranchName
        reviewDialog.prepare(backend.forgeDefaultTargetBranch, title)
        forgeReviewDialogVisible = true
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.canvas
    }

    Item {
        id: toolbar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 66

        Column {
            anchors.left: parent.left
            anchors.leftMargin: 20
            anchors.verticalCenter: parent.verticalCenter
            spacing: 3

            Text {
                text: root.backend.gitBranchName || "Repository"
                color: Theme.foreground
                font.family: Theme.uiFont
                font.pixelSize: 17
                font.weight: Font.DemiBold
            }

            Text {
                text: {
                    if (!root.backend.gitBranchHasUpstream)
                        return "Local branch · not published"
                    return "↑ " + root.backend.gitBranchAheadCount + " ahead  ↓ "
                        + root.backend.gitBranchBehindCount + " behind"
                }
                color: Theme.muted
                font.family: Theme.monoFont
                font.pixelSize: 10
            }
        }

        Row {
            anchors.right: parent.right
            anchors.rightMargin: 20
            anchors.verticalCenter: parent.verticalCenter
            spacing: 7

            ActionButton {
                label: "Refresh"
                enabled: !root.backend.gitBusy && !root.backend.gitLoading
                onClicked: root.backend.refresh_git_workspace()
            }

            ActionButton {
                visible: !root.backend.gitBranchHasUpstream
                label: "Publish"
                primary: true
                enabled: root.backend.gitReady && !root.backend.gitBusy && !root.backend.gitLoading
                onClicked: root.backend.publish_branch()
            }

            ActionButton {
                visible: root.backend.gitBranchHasUpstream
                label: "Pull --rebase"
                enabled: root.backend.gitReady && !root.backend.gitBusy && !root.backend.gitLoading
                onClicked: root.backend.pull_branch_with_rebase()
            }

            ActionButton {
                visible: root.backend.gitBranchHasUpstream
                label: "Sync"
                enabled: root.backend.gitReady && !root.backend.gitBusy && !root.backend.gitLoading
                onClicked: root.backend.sync_branch()
            }

            ActionButton {
                visible: root.backend.gitBranchHasUpstream
                label: "Push"
                primary: true
                enabled: root.backend.gitReady && !root.backend.gitBusy && !root.backend.gitLoading
                onClicked: root.backend.push_branch()
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
        anchors.top: toolbar.bottom
        height: visible ? 34 : 0
        visible: root.backend.gitError.length > 0
            || root.backend.gitStatusMessage.length > 0
            || root.backend.gitBusy
            || root.backend.gitLoading
        color: root.backend.gitError.length > 0 ? Theme.negativeMuted : Theme.raised

        Text {
            anchors.left: parent.left
            anchors.right: dismissStatus.left
            anchors.leftMargin: 20
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            text: root.backend.gitError.length > 0
                ? root.backend.gitError
                : (root.backend.gitBusy ? root.backend.gitActionLabel + "…"
                    : (root.backend.gitLoading ? "Loading repository state…"
                        : root.backend.gitStatusMessage))
            color: root.backend.gitError.length > 0 ? Theme.negative : Theme.muted
            elide: Text.ElideRight
            font.family: Theme.uiFont
            font.pixelSize: 11
        }

        Text {
            id: dismissStatus
            anchors.right: parent.right
            anchors.rightMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            text: root.backend.gitBusy ? "" : (root.backend.gitLoading ? "LOADING" : "READY")
            color: Theme.faint
            font.family: Theme.monoFont
            font.pixelSize: 9
        }
    }

    Item {
        id: content
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: statusBanner.bottom
        anchors.bottom: parent.bottom

        Item {
            id: changesPane
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: Math.max(420, parent.width - 342)

            Item {
                id: changesHeader
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                height: 48

                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 20
                    anchors.verticalCenter: parent.verticalCenter
                    text: "CHANGES  " + root.backend.gitChangedFileCount
                    color: Theme.foreground
                    font.family: Theme.uiFont
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.5
                }

                Row {
                    anchors.right: parent.right
                    anchors.rightMargin: 14
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 7

                    ActionButton {
                        label: "Unstage all"
                        compact: true
                        enabled: root.backend.gitStagedFileCount > 0
                            && !root.backend.gitBusy && !root.backend.gitLoading
                        onClicked: root.backend.unstage_all()
                    }

                    ActionButton {
                        label: "Stage all"
                        compact: true
                        primary: true
                        enabled: root.backend.gitUnstagedFileCount > 0
                            && !root.backend.gitBusy && !root.backend.gitLoading
                        onClicked: root.backend.stage_all()
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
                id: fileList
                objectName: "gitFileList"
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: changesHeader.bottom
                anchors.bottom: parent.bottom
                clip: true
                model: root.backend.gitFiles
                boundsBehavior: Flickable.StopAtBounds
                reuseItems: true
                cacheBuffer: Theme.fileRowHeight * 4
                section.property: "section"
                section.criteria: ViewSection.FullString

                section.delegate: Rectangle {
                    required property string section
                    width: fileList.width
                    height: 28
                    color: Theme.chrome

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 20
                        anchors.verticalCenter: parent.verticalCenter
                        text: parent.section
                        color: Theme.muted
                        font.family: Theme.monoFont
                        font.pixelSize: 9
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.9
                    }
                }

                delegate: Rectangle {
                    id: fileRow

                    required property string path
                    required property string file_name
                    required property string directory
                    required property string status_tag
                    required property string status_label
                    required property bool staged
                    required property int additions
                    required property int removals

                    width: fileList.width
                    height: Theme.fileRowHeight
                    color: filePointer.containsMouse ? Theme.hover : Theme.transparent

                    Rectangle {
                        anchors.left: parent.left
                        anchors.leftMargin: 20
                        anchors.verticalCenter: parent.verticalCenter
                        width: 24
                        height: 24
                        radius: 4
                        color: fileRow.status_tag === "D" || fileRow.status_tag === "!"
                            ? Theme.negativeMuted : Theme.accentMuted

                        Text {
                            anchors.centerIn: parent
                            text: fileRow.status_tag
                            color: fileRow.status_tag === "D" || fileRow.status_tag === "!"
                                ? Theme.negative : Theme.accentStrong
                            font.family: Theme.monoFont
                            font.pixelSize: 10
                            font.weight: Font.Bold
                        }
                    }

                    Column {
                        anchors.left: parent.left
                        anchors.leftMargin: 54
                        anchors.right: stats.left
                        anchors.rightMargin: 12
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 2

                        Text {
                            width: parent.width
                            text: fileRow.file_name
                            color: Theme.foreground
                            elide: Text.ElideRight
                            font.family: Theme.uiFont
                            font.pixelSize: 12
                            font.weight: Font.Medium
                        }

                        Text {
                            width: parent.width
                            text: fileRow.directory.length > 0 ? fileRow.directory : fileRow.status_label
                            color: Theme.faint
                            elide: Text.ElideLeft
                            font.family: Theme.monoFont
                            font.pixelSize: 9
                        }
                    }

                    Row {
                        id: stats
                        anchors.right: rowActions.left
                        anchors.rightMargin: 12
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 7

                        Text {
                            text: fileRow.additions > 0 ? "+" + fileRow.additions : ""
                            color: Theme.positive
                            font.family: Theme.monoFont
                            font.pixelSize: 10
                        }

                        Text {
                            text: fileRow.removals > 0 ? "−" + fileRow.removals : ""
                            color: Theme.negative
                            font.family: Theme.monoFont
                            font.pixelSize: 10
                        }
                    }

                    Row {
                        id: rowActions
                        anchors.right: parent.right
                        anchors.rightMargin: 14
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 6

                        ActionButton {
                            visible: !fileRow.staged
                            label: "Discard"
                            compact: true
                            danger: true
                            enabled: !root.backend.gitBusy && !root.backend.gitLoading
                            onClicked: root.requestDiscard(fileRow.path)
                        }

                        ActionButton {
                            label: fileRow.staged ? "Unstage" : "Stage"
                            compact: true
                            primary: true
                            enabled: !root.backend.gitBusy && !root.backend.gitLoading
                            onClicked: {
                                if (fileRow.staged)
                                    root.backend.unstage_path(fileRow.path)
                                else
                                    root.backend.stage_path(fileRow.path)
                            }
                        }
                    }

                    MouseArea {
                        id: filePointer
                        anchors.left: parent.left
                        anchors.right: rowActions.left
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        hoverEnabled: true
                        acceptedButtons: Qt.NoButton
                    }

                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 1
                        color: Theme.border
                    }
                }
            }

            Column {
                anchors.centerIn: fileList
                width: Math.min(360, fileList.width - 48)
                spacing: 8
                visible: root.emptyStateVisible

                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: "Working tree clean"
                    color: Theme.foreground
                    font.family: Theme.uiFont
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
                }

                Text {
                    width: parent.width
                    text: root.backend.gitLastCommitSubject.length > 0
                        ? "Latest commit · " + root.backend.gitLastCommitSubject
                        : "There are no local changes to review."
                    color: Theme.muted
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode: Text.WordWrap
                    font.family: Theme.uiFont
                    font.pixelSize: 11
                }
            }
        }

        Rectangle {
            id: detailsPane
            anchors.left: changesPane.right
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            color: Theme.chrome

            Rectangle {
                anchors.left: parent.left
                width: 1
                height: parent.height
                color: Theme.border
            }

            Column {
                id: commitComposer
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: 16
                spacing: 10

                Text {
                    text: "COMMIT"
                    color: Theme.muted
                    font.family: Theme.uiFont
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 1
                }

                Rectangle {
                    width: parent.width
                    height: 78
                    radius: 5
                    color: Theme.input
                    border.width: commitMessage.activeFocus ? 1 : 0
                    border.color: Theme.accentStrong

                    TextEdit {
                        id: commitMessage
                        objectName: "gitCommitMessage"
                        anchors.fill: parent
                        anchors.margins: 10
                        color: Theme.foreground
                        selectionColor: Theme.accent
                        selectedTextColor: Theme.foreground
                        wrapMode: TextEdit.Wrap
                        selectByMouse: true
                        font.family: Theme.uiFont
                        font.pixelSize: 12

                        Keys.onPressed: event => {
                            if ((event.modifiers & (Qt.ControlModifier | Qt.MetaModifier))
                                    && (event.key === Qt.Key_Return || event.key === Qt.Key_Enter)) {
                                root.submitCommit()
                                event.accepted = true
                            }
                        }
                    }

                    Text {
                        anchors.left: parent.left
                        anchors.top: parent.top
                        anchors.margins: 10
                        visible: commitMessage.text.length === 0 && !commitMessage.activeFocus
                        text: "Commit message"
                        color: Theme.faint
                        font.family: Theme.uiFont
                        font.pixelSize: 12
                    }
                }

                Row {
                    width: parent.width

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width - commitButton.width
                        text: root.backend.gitStagedFileCount + " staged"
                        color: Theme.muted
                        font.family: Theme.monoFont
                        font.pixelSize: 10
                    }

                    ActionButton {
                        id: commitButton
                        label: "Commit"
                        primary: true
                        enabled: commitMessage.text.trim().length > 0
                            && root.backend.gitStagedFileCount > 0
                            && !root.backend.gitBusy && !root.backend.gitLoading
                        onClicked: root.submitCommit()
                    }
                }
            }

            Rectangle {
                id: composerDivider
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: commitComposer.bottom
                anchors.topMargin: 16
                height: 1
                color: Theme.border
            }

            ForgePanel {
                id: forgePanel
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: composerDivider.bottom
                height: implicitHeight
                backend: root.backend
                onAuthenticationRequested: root.requestForgeAuthentication()
                onReviewRequested: root.openForgeReviewDialog()
            }

            Rectangle {
                id: forgeDivider
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: forgePanel.bottom
                height: 1
                color: Theme.border
            }

            Text {
                id: historyHeading
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: forgeDivider.bottom
                height: 42
                leftPadding: 16
                verticalAlignment: Text.AlignVCenter
                text: "RECENT COMMITS"
                color: Theme.muted
                font.family: Theme.uiFont
                font.pixelSize: 10
                font.weight: Font.DemiBold
                font.letterSpacing: 1
            }

            ListView {
                id: commitList
                objectName: "gitCommitList"
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: historyHeading.bottom
                anchors.bottom: parent.bottom
                clip: true
                model: root.backend.gitCommits
                boundsBehavior: Flickable.StopAtBounds
                reuseItems: true
                cacheBuffer: 160

                delegate: Rectangle {
                    id: commitRow

                    required property string short_id
                    required property string subject

                    width: commitList.width
                    height: 54
                    color: Theme.transparent

                    Column {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.leftMargin: 16
                        anchors.rightMargin: 14
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 4

                        Text {
                            width: parent.width
                            text: commitRow.subject
                            color: Theme.foreground
                            elide: Text.ElideRight
                            font.family: Theme.uiFont
                            font.pixelSize: 11
                        }

                        Text {
                            text: commitRow.short_id
                            color: Theme.faint
                            font.family: Theme.monoFont
                            font.pixelSize: 9
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
        }
    }

    Rectangle {
        anchors.fill: content
        visible: root.loadingStateVisible
        z: 20
        color: Theme.canvas
    }

    ConfirmationDialog {
        id: discardDialog
        objectName: "gitDiscardConfirmation"
        anchors.fill: parent
        visible: root.discardConfirmationVisible
        title: "Discard local changes?"
        message: "This permanently restores “" + root.pendingDiscardPath
            + "” to its last committed state. Untracked content will be removed."
        confirmLabel: "Discard"
        onAccepted: root.confirmDiscard()
        onRejected: root.cancelDiscard()
    }

    ForgeTokenDialog {
        id: tokenDialog
        objectName: "forgeTokenDialog"
        anchors.fill: parent
        visible: root.forgeTokenDialogVisible
        providerLabel: root.backend.forgeProviderLabel
        onSubmitted: token => {
            root.backend.save_forge_personal_access_token(token)
            root.forgeTokenDialogVisible = false
            clear()
        }
        onRejected: {
            root.forgeTokenDialogVisible = false
            clear()
        }
    }

    ForgeReviewDialog {
        id: reviewDialog
        objectName: "forgeReviewDialog"
        anchors.fill: parent
        visible: root.forgeReviewDialogVisible
        providerLabel: root.backend.forgeProviderLabel
        reviewKindLabel: root.backend.forgeReviewKindLabel
        onSubmitted: (targetBranch, title, body, draft) => {
            root.backend.create_forge_review(targetBranch, title, body, draft)
            root.forgeReviewDialogVisible = false
        }
        onRejected: root.forgeReviewDialogVisible = false
    }
}
