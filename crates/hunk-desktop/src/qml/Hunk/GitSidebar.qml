pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Dialogs

Item {
    id: root

    required property var backend

    Rectangle {
        anchors.fill: parent
        color: Theme.chrome
    }

    Column {
        id: repositoryHeader
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 16
        spacing: 5

        Row {
            width: parent.width

            Text {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - openRepository.width
                text: "REPOSITORY"
                color: Theme.muted
                font.family: Theme.uiFont
                font.pixelSize: 10
                font.weight: Font.DemiBold
                font.letterSpacing: 1.1
            }

            ActionButton {
                id: openRepository
                label: "Open"
                compact: true
                enabled: !root.backend.gitBusy && !root.backend.gitLoading
                onClicked: repositoryDialog.open()
            }
        }

        Text {
            width: parent.width
            text: root.backend.gitRepositoryName || "Loading repository…"
            color: Theme.foreground
            elide: Text.ElideMiddle
            font.family: Theme.uiFont
            font.pixelSize: 15
            font.weight: Font.DemiBold
        }

        Text {
            width: parent.width
            text: root.backend.gitRoot || ""
            color: Theme.faint
            elide: Text.ElideMiddle
            font.family: Theme.monoFont
            font.pixelSize: 10
        }
    }

    FolderDialog {
        id: repositoryDialog
        title: "Open Git repository"
        onAccepted: root.backend.select_git_root(selectedFolder.toString())
    }

    Row {
        id: branchHeading
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: repositoryHeader.bottom
        anchors.topMargin: 24
        anchors.leftMargin: 16
        anchors.rightMargin: 12
        height: 24

        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: "BRANCHES"
            color: Theme.muted
            font.family: Theme.uiFont
            font.pixelSize: 10
            font.weight: Font.DemiBold
            font.letterSpacing: 1.1
        }

        Item { width: parent.width - parent.children[0].width - refreshBranches.width; height: 1 }

        ActionButton {
            id: refreshBranches
            anchors.verticalCenter: parent.verticalCenter
            label: "Fetch"
            compact: true
            enabled: !root.backend.gitBusy && !root.backend.gitLoading
            onClicked: root.backend.fetch_remote_branches()
        }
    }

    ListView {
        id: branchList
        objectName: "gitBranchList"
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: branchHeading.bottom
        anchors.bottom: parent.bottom
        anchors.topMargin: 4
        clip: true
        model: root.backend.gitBranches
        boundsBehavior: Flickable.StopAtBounds
        reuseItems: true
        cacheBuffer: Theme.compactRowHeight * 3

        delegate: Rectangle {
            id: branchRow

            required property string name
            required property bool current
            required property bool remote
            required property string workspace_label

            width: branchList.width
            height: Theme.compactRowHeight
            color: current ? Theme.selected : (branchPointer.containsMouse ? Theme.hover : Theme.transparent)

            Rectangle {
                anchors.left: parent.left
                anchors.verticalCenter: parent.verticalCenter
                width: 2
                height: 20
                color: branchRow.current ? Theme.accentStrong : Theme.transparent
            }

            Text {
                anchors.left: parent.left
                anchors.right: remoteLabel.left
                anchors.leftMargin: 16
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                text: branchRow.name
                color: branchRow.current ? Theme.foreground : Theme.muted
                elide: Text.ElideMiddle
                font.family: Theme.monoFont
                font.pixelSize: 11
                font.weight: branchRow.current ? Font.DemiBold : Font.Normal
            }

            Text {
                id: remoteLabel
                anchors.right: parent.right
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                text: branchRow.remote ? "REMOTE" : ""
                color: Theme.faint
                font.family: Theme.monoFont
                font.pixelSize: 8
                font.letterSpacing: 0.7
            }

            MouseArea {
                id: branchPointer
                anchors.fill: parent
                enabled: !branchRow.current && !root.backend.gitBusy && !root.backend.gitLoading
                hoverEnabled: true
                cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                onClicked: root.backend.activate_branch(branchRow.name)
            }
        }
    }
}
