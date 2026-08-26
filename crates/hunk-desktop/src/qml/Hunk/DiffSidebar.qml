pragma ComponentBehavior: Bound

import QtQuick

Rectangle {
    id: root

    required property var backend
    readonly property alias fileListView: fileList

    function activateFile(path) {
        backend.select_diff_file(path)
    }

    color: Theme.chrome

    Column {
        id: heading
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 16
        spacing: 5

        Text {
            text: "CHANGED FILES"
            color: Theme.muted
            font.family: Theme.uiFont
            font.pixelSize: 10
            font.weight: Font.DemiBold
            font.letterSpacing: 1.1
        }

        Text {
            width: parent.width
            text: root.backend.gitRepositoryName
            color: Theme.foreground
            elide: Text.ElideRight
            font.family: Theme.uiFont
            font.pixelSize: 14
            font.weight: Font.DemiBold
        }

        Row {
            width: parent.width
            spacing: 7

            Text {
                width: Math.max(0, parent.width - fileCount.width - parent.spacing)
                text: root.backend.diffCompareLeftLabel.length > 0
                    ? root.backend.diffCompareLeftLabel + " → "
                        + root.backend.diffCompareRightLabel
                    : (root.backend.gitBranchName.length > 0
                        ? root.backend.gitBranchName : "No branch")
                textFormat: Text.PlainText
                color: Theme.faint
                elide: Text.ElideMiddle
                font.family: Theme.monoFont
                font.pixelSize: 9
            }

            Text {
                id: fileCount
                text: "· " + root.backend.diffCompareFileCount
                color: Theme.faint
                font.family: Theme.monoFont
                font.pixelSize: 9
            }
        }
    }

    Rectangle {
        id: divider
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: heading.bottom
        anchors.topMargin: 14
        height: 1
        color: Theme.border
    }

    ListView {
        id: fileList
        objectName: "diffFileList"
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: divider.bottom
        anchors.bottom: parent.bottom
        clip: true
        model: root.backend.diffFiles
        boundsBehavior: Flickable.StopAtBounds
        reuseItems: true
        cacheBuffer: Theme.compactRowHeight * 5

        delegate: Rectangle {
            id: fileRow

            required property string path
            required property string file_name
            required property string directory
            required property string status_tag
            required property int additions
            required property int removals

            width: fileList.width
            height: Theme.compactRowHeight + 8
            color: root.backend.diffSelectedPath === fileRow.path
                ? Theme.selected : (pointer.containsMouse ? Theme.hover : Theme.transparent)

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: 2
                color: root.backend.diffSelectedPath === fileRow.path
                    ? Theme.accentStrong : Theme.transparent
            }

            Text {
                id: status
                anchors.left: parent.left
                anchors.leftMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                width: 18
                text: fileRow.status_tag
                color: fileRow.status_tag === "D" || fileRow.status_tag === "!"
                    ? Theme.negative : Theme.accentStrong
                font.family: Theme.monoFont
                font.pixelSize: 10
                font.weight: Font.Bold
            }

            Column {
                anchors.left: status.right
                anchors.leftMargin: 5
                anchors.right: stats.left
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2

                Text {
                    width: parent.width
                    text: fileRow.file_name
                    color: Theme.foreground
                    elide: Text.ElideRight
                    font.family: Theme.uiFont
                    font.pixelSize: 11
                    font.weight: Font.Medium
                }

                Text {
                    width: parent.width
                    text: fileRow.directory.length > 0 ? fileRow.directory : "repository root"
                    color: Theme.faint
                    elide: Text.ElideLeft
                    font.family: Theme.monoFont
                    font.pixelSize: 8
                }
            }

            Row {
                id: stats
                anchors.right: parent.right
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4

                Text {
                    text: "+" + fileRow.additions
                    color: Theme.positive
                    font.family: Theme.monoFont
                    font.pixelSize: 9
                }

                Text {
                    text: "−" + fileRow.removals
                    color: Theme.negative
                    font.family: Theme.monoFont
                    font.pixelSize: 9
                }
            }

            MouseArea {
                id: pointer
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.activateFile(fileRow.path)
            }
        }
    }
}
