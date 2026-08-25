pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property var backend
    readonly property alias diffListView: diffList
    readonly property bool loadingStateVisible: loadingState.visible
    readonly property bool errorStateVisible: errorState.visible
    readonly property bool emptyStateVisible: emptyState.visible

    function cellColor(kind) {
        if (kind === "added")
            return Theme.positiveMuted
        if (kind === "removed")
            return Theme.negativeMuted
        return Theme.canvas
    }

    function markerColor(kind) {
        if (kind === "added")
            return Theme.positive
        if (kind === "removed")
            return Theme.negative
        return Theme.faint
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
        height: 52

        Column {
            anchors.left: parent.left
            anchors.leftMargin: 18
            anchors.right: actions.left
            anchors.rightMargin: 18
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2

            Text {
                width: parent.width
                text: root.backend.diffSelectedPath.length > 0
                    ? root.backend.diffSelectedPath : "Working tree"
                color: Theme.foreground
                elide: Text.ElideMiddle
                font.family: Theme.monoFont
                font.pixelSize: 12
                font.weight: Font.DemiBold
            }

            Row {
                spacing: 8

                Text {
                    text: root.backend.diffStatusTag.length > 0
                        ? root.backend.diffStatusTag : "—"
                    color: Theme.accentStrong
                    font.family: Theme.monoFont
                    font.pixelSize: 9
                    font.weight: Font.Bold
                }

                Text {
                    text: "+" + root.backend.diffAdditions
                    color: Theme.positive
                    font.family: Theme.monoFont
                    font.pixelSize: 9
                }

                Text {
                    text: "−" + root.backend.diffRemovals
                    color: Theme.negative
                    font.family: Theme.monoFont
                    font.pixelSize: 9
                }
            }
        }

        Row {
            id: actions
            anchors.right: parent.right
            anchors.rightMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            spacing: 7

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: root.backend.diffLoading ? "LOADING" : "SIDE BY SIDE"
                color: Theme.faint
                font.family: Theme.monoFont
                font.pixelSize: 9
                font.letterSpacing: 0.7
            }

            ActionButton {
                label: "Refresh"
                compact: true
                enabled: root.backend.diffSelectedPath.length > 0 && !root.backend.diffLoading
                onClicked: root.backend.refresh_diff()
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
        id: columnHeader
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: toolbar.bottom
        height: 28
        visible: root.backend.diffSelectedPath.length > 0

        Rectangle {
            anchors.fill: parent
            color: Theme.chrome
        }

        Text {
            anchors.left: parent.left
            anchors.leftMargin: 58
            anchors.verticalCenter: parent.verticalCenter
            text: "BEFORE"
            color: Theme.muted
            font.family: Theme.monoFont
            font.pixelSize: 9
            font.weight: Font.DemiBold
            font.letterSpacing: 0.8
        }

        Text {
            anchors.left: parent.horizontalCenter
            anchors.leftMargin: 58
            anchors.verticalCenter: parent.verticalCenter
            text: "AFTER"
            color: Theme.muted
            font.family: Theme.monoFont
            font.pixelSize: 9
            font.weight: Font.DemiBold
            font.letterSpacing: 0.8
        }

        Rectangle {
            anchors.left: parent.horizontalCenter
            width: 1
            height: parent.height
            color: Theme.borderStrong
        }

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.border
        }
    }

    Flickable {
        id: horizontalViewport
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: columnHeader.bottom
        anchors.bottom: parent.bottom
        contentWidth: Math.max(width, 1440)
        contentHeight: height
        flickableDirection: Flickable.HorizontalFlick
        boundsBehavior: Flickable.StopAtBounds
        clip: true

        ListView {
            id: diffList
            objectName: "diffRowList"
            width: horizontalViewport.contentWidth
            height: horizontalViewport.height
            model: root.backend.diffRows
            clip: true
            reuseItems: true
            cacheBuffer: Theme.diffRowHeight * 12
            boundsBehavior: Flickable.StopAtBounds

            delegate: Item {
                id: diffRow

                required property string stable_id
                required property string row_kind
                required property int left_line
                required property string left_text
                required property string left_kind
                required property int right_line
                required property string right_text
                required property string right_kind
                required property string text

                width: diffList.width
                height: diffRow.row_kind === "code" ? Theme.diffRowHeight
                    : (diffRow.row_kind === "hunk" ? 32 : 54)

                Rectangle {
                    anchors.fill: parent
                    visible: diffRow.row_kind !== "code"
                    color: diffRow.row_kind === "hunk" ? Theme.raised : Theme.canvas

                    Text {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.leftMargin: 18
                        anchors.rightMargin: 18
                        anchors.verticalCenter: parent.verticalCenter
                        text: diffRow.text
                        color: diffRow.row_kind === "hunk" ? Theme.accentStrong : Theme.muted
                        elide: Text.ElideRight
                        font.family: Theme.monoFont
                        font.pixelSize: diffRow.row_kind === "hunk" ? 10 : 11
                    }

                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 1
                        color: Theme.border
                    }
                }

                Item {
                    anchors.fill: parent
                    visible: diffRow.row_kind === "code"

                    Rectangle {
                        anchors.left: parent.left
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        width: parent.width / 2
                        color: root.cellColor(diffRow.left_kind)
                    }

                    Rectangle {
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        width: parent.width / 2
                        color: root.cellColor(diffRow.right_kind)
                    }

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 8
                        anchors.verticalCenter: parent.verticalCenter
                        width: 38
                        text: diffRow.left_line > 0 ? diffRow.left_line : ""
                        color: Theme.faint
                        horizontalAlignment: Text.AlignRight
                        font.family: Theme.monoFont
                        font.pixelSize: 9
                    }

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 52
                        anchors.verticalCenter: parent.verticalCenter
                        text: diffRow.left_kind === "removed" ? "−" : " "
                        color: root.markerColor(diffRow.left_kind)
                        font.family: Theme.monoFont
                        font.pixelSize: 11
                    }

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 68
                        anchors.verticalCenter: parent.verticalCenter
                        text: diffRow.left_text
                        color: Theme.foreground
                        textFormat: Text.PlainText
                        font.family: Theme.monoFont
                        font.pixelSize: 11
                    }

                    Text {
                        anchors.left: parent.horizontalCenter
                        anchors.leftMargin: 8
                        anchors.verticalCenter: parent.verticalCenter
                        width: 38
                        text: diffRow.right_line > 0 ? diffRow.right_line : ""
                        color: Theme.faint
                        horizontalAlignment: Text.AlignRight
                        font.family: Theme.monoFont
                        font.pixelSize: 9
                    }

                    Text {
                        anchors.left: parent.horizontalCenter
                        anchors.leftMargin: 52
                        anchors.verticalCenter: parent.verticalCenter
                        text: diffRow.right_kind === "added" ? "+" : " "
                        color: root.markerColor(diffRow.right_kind)
                        font.family: Theme.monoFont
                        font.pixelSize: 11
                    }

                    Text {
                        anchors.left: parent.horizontalCenter
                        anchors.leftMargin: 68
                        anchors.verticalCenter: parent.verticalCenter
                        text: diffRow.right_text
                        color: Theme.foreground
                        textFormat: Text.PlainText
                        font.family: Theme.monoFont
                        font.pixelSize: 11
                    }

                    Rectangle {
                        anchors.left: parent.horizontalCenter
                        width: 1
                        height: parent.height
                        color: Theme.borderStrong
                    }
                }
            }
        }
    }

    Text {
        id: loadingState
        anchors.centerIn: horizontalViewport
        visible: root.backend.diffLoading
        text: "Loading diff…"
        color: Theme.muted
        font.family: Theme.uiFont
        font.pixelSize: 12
    }

    Text {
        id: errorState
        anchors.centerIn: horizontalViewport
        width: Math.min(520, horizontalViewport.width - 48)
        visible: root.backend.diffError.length > 0
        text: root.backend.diffError
        color: Theme.negative
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
        font.family: Theme.uiFont
        font.pixelSize: 12
    }

    Text {
        id: emptyState
        anchors.centerIn: horizontalViewport
        visible: !root.backend.diffLoading
            && root.backend.diffError.length === 0
            && root.backend.diffSelectedPath.length === 0
        text: "Working tree is clean"
        color: Theme.muted
        font.family: Theme.uiFont
        font.pixelSize: 12
    }
}
