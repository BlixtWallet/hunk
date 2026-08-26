pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic

Item {
    id: root

    required property var model
    required property int currentIndex
    required property string currentLabel
    property string accessibleName: qsTr("Comparison source")
    readonly property alias listView: sourceList
    readonly property alias popup: sourceMenu
    signal selected(int index)

    implicitWidth: 220
    implicitHeight: 28

    ActionButton {
        anchors.fill: parent
        label: root.currentLabel.length > 0 ? root.currentLabel : qsTr("Select source")
        accessibleName: root.accessibleName
        compact: true
        onClicked: sourceMenu.open()
    }

    Popup {
        id: sourceMenu

        x: 0
        y: root.height + 4
        width: root.width
        height: Math.min(320, Math.max(42, sourceList.contentHeight + 8))
        padding: 4
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            color: Theme.raised
            border.width: 1
            border.color: Theme.borderStrong
            radius: 6
        }

        contentItem: ListView {
            id: sourceList

            clip: true
            model: root.model
            currentIndex: root.currentIndex
            boundsBehavior: Flickable.StopAtBounds
            reuseItems: true
            focus: sourceMenu.opened

            function chooseCurrent() {
                if (currentIndex < 0 || currentIndex >= count)
                    return
                root.selected(currentIndex)
                sourceMenu.close()
            }

            Keys.onReturnPressed: event => {
                sourceList.chooseCurrent()
                event.accepted = true
            }
            Keys.onEnterPressed: event => {
                sourceList.chooseCurrent()
                event.accepted = true
            }
            Keys.onSpacePressed: event => {
                sourceList.chooseCurrent()
                event.accepted = true
            }

            delegate: Rectangle {
                id: sourceRow

                required property int index
                required property string label
                required property string detail

                width: sourceList.width
                height: 44
                radius: 4
                color: sourceRow.index === root.currentIndex
                    ? Theme.selected : (pointer.hovered ? Theme.hover : Theme.transparent)
                activeFocusOnTab: true
                Accessible.role: Accessible.ListItem
                Accessible.name: sourceRow.label + ", " + sourceRow.detail
                Accessible.onPressAction: sourceRow.choose()

                function choose() {
                    root.selected(sourceRow.index)
                    sourceMenu.close()
                }

                Keys.onReturnPressed: sourceRow.choose()
                Keys.onEnterPressed: sourceRow.choose()
                Keys.onSpacePressed: sourceRow.choose()

                Column {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.leftMargin: 10
                    anchors.rightMargin: 10
                    spacing: 2

                    Text {
                        width: parent.width
                        text: sourceRow.label
                        textFormat: Text.PlainText
                        color: Theme.foreground
                        elide: Text.ElideMiddle
                        font.family: Theme.uiFont
                        font.pixelSize: 11
                        font.weight: Font.Medium
                    }

                    Text {
                        width: parent.width
                        text: sourceRow.detail
                        textFormat: Text.PlainText
                        color: Theme.faint
                        elide: Text.ElideRight
                        font.family: Theme.uiFont
                        font.pixelSize: 9
                    }
                }

                HoverHandler {
                    id: pointer
                    cursorShape: Qt.PointingHandCursor
                }

                TapHandler {
                    onTapped: sourceRow.choose()
                }
            }
        }
    }
}
