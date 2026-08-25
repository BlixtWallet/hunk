pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property QtObject backend
    readonly property alias screen: terminalScreen
    property real dragStartY: 0
    property real dragStartHeight: 0
    signal resizeRequested(real height)
    signal closeRequested

    function focusTerminal() {
        terminalScreen.focusTerminal()
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.terminalBackground
        border.width: 1
        border.color: Theme.border
    }

    MouseArea {
        id: resizeHandle
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 7
        hoverEnabled: true
        cursorShape: Qt.SizeVerCursor
        onPressed: mouse => {
            root.dragStartY = resizeHandle.mapToItem(null, mouse.x, mouse.y).y
            root.dragStartHeight = root.height
        }
        onPositionChanged: mouse => {
            if (pressed) {
                const sceneY = resizeHandle.mapToItem(null, mouse.x, mouse.y).y
                root.resizeRequested(root.dragStartHeight + root.dragStartY - sceneY)
            }
        }

        Rectangle {
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.verticalCenter: parent.verticalCenter
            width: 42
            height: 2
            radius: 1
            color: resizeHandle.containsMouse ? Theme.accent : Theme.borderStrong
        }
    }

    Row {
        id: tabBar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: resizeHandle.bottom
        height: 34
        spacing: 6

        ListView {
            id: tabs
            objectName: "terminalTabs"
            width: Math.max(0, parent.width - newTab.width - 22)
            height: parent.height
            orientation: ListView.Horizontal
            spacing: 5
            leftMargin: 8
            model: root.backend.terminalTabs
            currentIndex: root.backend.terminalActiveTabIndex
            reuseItems: true
            clip: true
            onCurrentIndexChanged: {
                if (currentIndex >= 0)
                    Qt.callLater(() => tabs.positionViewAtIndex(
                        tabs.currentIndex, ListView.Contain))
            }

            delegate: Item {
                id: tab
                required property int tab_id
                required property string title
                required property string status
                readonly property bool selected: tab_id === root.backend.terminalActiveTabId
                width: Math.min(160, Math.max(88, titleLabel.implicitWidth + 50))
                height: tabs.height
                activeFocusOnTab: true
                Accessible.role: Accessible.PageTab
                Accessible.name: title

                Rectangle {
                    anchors.fill: parent
                    anchors.topMargin: 4
                    anchors.bottomMargin: 4
                    radius: 5
                    color: tab.selected ? Theme.selected
                        : (tabPointer.containsMouse ? Theme.hover : Theme.transparent)
                    border.width: tab.selected || tab.activeFocus ? 1 : 0
                    border.color: tab.activeFocus ? Theme.accentStrong : Theme.border
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.leftMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    width: 6
                    height: 6
                    radius: 3
                    color: tab.status === "running" ? Theme.positive
                        : (tab.status === "failed" ? Theme.negative : Theme.muted)
                }

                Text {
                    id: titleLabel
                    anchors.left: parent.left
                    anchors.leftMargin: 23
                    anchors.right: closeTab.left
                    anchors.rightMargin: 5
                    anchors.verticalCenter: parent.verticalCenter
                    text: tab.title
                    textFormat: Text.PlainText
                    color: tab.selected ? Theme.foreground : Theme.muted
                    elide: Text.ElideRight
                    font.family: Theme.uiFont
                    font.pixelSize: 11
                    font.weight: tab.selected ? Font.DemiBold : Font.Normal
                }

                Text {
                    id: closeTab
                    anchors.right: parent.right
                    anchors.rightMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    text: "×"
                    color: closePointer.containsMouse ? Theme.foreground : Theme.faint
                    font.family: Theme.uiFont
                    font.pixelSize: 14

                    MouseArea {
                        id: closePointer
                        anchors.fill: parent
                        anchors.margins: -6
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: mouse => {
                            mouse.accepted = true
                            root.backend.close_terminal_tab(tab.tab_id)
                        }
                    }
                }

                MouseArea {
                    id: tabPointer
                    anchors.fill: parent
                    anchors.rightMargin: 27
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.backend.select_terminal_tab(tab.tab_id)
                }

                Keys.onReturnPressed: event => {
                    root.backend.select_terminal_tab(tab.tab_id)
                    event.accepted = true
                }
                Keys.onSpacePressed: event => {
                    root.backend.select_terminal_tab(tab.tab_id)
                    event.accepted = true
                }
            }
        }

        ActionButton {
            id: newTab
            anchors.verticalCenter: parent.verticalCenter
            label: "+"
            accessibleName: qsTr("New terminal tab")
            compact: true
            onClicked: root.backend.new_terminal_tab()
        }
    }

    Item {
        id: toolbar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: tabBar.bottom
        height: 32

        Rectangle {
            anchors.fill: parent
            color: Theme.chrome
        }

        Row {
            anchors.left: parent.left
            anchors.leftMargin: 10
            anchors.right: toolbarActions.left
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            spacing: 8
            clip: true

            Text {
                text: qsTr("Terminal")
                color: Theme.foreground
                font.family: Theme.uiFont
                font.pixelSize: 11
                font.weight: Font.DemiBold
                textFormat: Text.PlainText
            }

            Text {
                width: Math.min(120, implicitWidth)
                text: root.backend.terminalShellLabel
                textFormat: Text.PlainText
                color: Theme.muted
                elide: Text.ElideMiddle
                font.family: Theme.monoFont
                font.pixelSize: 10
            }

            Text {
                width: Math.max(0, parent.width - x)
                text: root.backend.terminalStatusMessage.length > 0
                    ? root.backend.terminalStatusMessage : root.backend.terminalCwd
                textFormat: Text.PlainText
                color: root.backend.terminalStatus === "failed"
                    ? Theme.negative : Theme.faint
                elide: Text.ElideMiddle
                font.family: Theme.monoFont
                font.pixelSize: 9
            }
        }

        Row {
            id: toolbarActions
            anchors.right: parent.right
            anchors.rightMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            spacing: 6

            ActionButton {
                label: qsTr("Clear")
                compact: true
                onClicked: root.backend.clear_terminal_screen()
            }

            ActionButton {
                label: qsTr("Close")
                compact: true
                onClicked: root.closeRequested()
            }
        }
    }

    TerminalScreen {
        id: terminalScreen
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: toolbar.bottom
        anchors.bottom: parent.bottom
        anchors.margins: 1
        backend: root.backend
    }

    Shortcut {
        enabled: root.visible
        sequence: Qt.platform.os === "osx" ? "Meta+T" : "Ctrl+Shift+T"
        autoRepeat: false
        onActivated: root.backend.new_terminal_tab()
    }

    Shortcut {
        enabled: root.visible
        sequence: Qt.platform.os === "osx" ? "Meta+W" : "Ctrl+Shift+W"
        autoRepeat: false
        onActivated: root.backend.close_terminal_tab(root.backend.terminalActiveTabId)
    }

    Shortcut {
        enabled: root.visible
        sequence: Qt.platform.os === "osx" ? "Meta+}" : "Ctrl+PgDown"
        autoRepeat: false
        onActivated: root.backend.move_terminal_tab(1)
    }

    Shortcut {
        enabled: root.visible
        sequence: Qt.platform.os === "osx" ? "Meta+{" : "Ctrl+PgUp"
        autoRepeat: false
        onActivated: root.backend.move_terminal_tab(-1)
    }
}
