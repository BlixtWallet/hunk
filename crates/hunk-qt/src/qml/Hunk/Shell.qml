import QtQuick

Item {
    id: root

    required property var backend
    readonly property var workspaceIds: ["diff", "git", "ai"]
    readonly property int workspaceCount: workspaceIds.length
    readonly property string activeWorkspace: backend.activeWorkspace

    function activateWorkspace(workspace) {
        backend.select_workspace(workspace)
    }

    function workspaceTitle(workspace) {
        if (workspace === "git")
            return "Repository"
        if (workspace === "ai")
            return "Codex"
        return "Review"
    }

    function workspaceDescription(workspace) {
        if (workspace === "git")
            return "Branches, changes, and commits"
        if (workspace === "ai")
            return "Threads, turns, and tools"
        return "Working tree and comparison diffs"
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.canvas
    }

    Rectangle {
        id: header
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Theme.headerHeight
        color: Theme.chrome

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.border
        }

        Row {
            anchors.left: parent.left
            anchors.leftMargin: 16
            height: parent.height
            spacing: 8

            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: "HUNK"
                color: Theme.foreground
                font.family: Theme.uiFont
                font.pixelSize: 13
                font.weight: Font.Bold
                font.letterSpacing: 1.2
            }

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 1
                height: 20
                color: Theme.border
            }

            Repeater {
                model: [
                    { label: "Diff", workspace: "diff" },
                    { label: "Git", workspace: "git" },
                    { label: "AI", workspace: "ai" }
                ]

                delegate: NavigationButton {
                    required property var modelData
                    label: modelData.label
                    workspace: modelData.workspace
                    selected: root.activeWorkspace === workspace
                    onActivated: workspace => root.activateWorkspace(workspace)
                }
            }
        }

        Row {
            anchors.right: parent.right
            anchors.rightMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            spacing: 8

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 7
                height: 7
                radius: 4
                color: root.backend.ready ? Theme.positive : Theme.muted
            }

            Text {
                text: root.backend.statusMessage
                color: Theme.muted
                font.family: Theme.uiFont
                font.pixelSize: 11
            }
        }
    }

    Rectangle {
        id: sidebar
        anchors.left: parent.left
        anchors.top: header.bottom
        anchors.bottom: parent.bottom
        width: Theme.sidebarWidth
        color: Theme.chrome

        Rectangle {
            anchors.right: parent.right
            width: 1
            height: parent.height
            color: Theme.border
        }

        Column {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 16
            spacing: 7

            Text {
                text: root.workspaceTitle(root.activeWorkspace).toUpperCase()
                color: Theme.muted
                font.family: Theme.uiFont
                font.pixelSize: 10
                font.weight: Font.DemiBold
                font.letterSpacing: 1.1
            }

            Text {
                width: parent.width
                text: root.workspaceDescription(root.activeWorkspace)
                color: Theme.foreground
                font.family: Theme.uiFont
                font.pixelSize: 14
                font.weight: Font.Medium
                wrapMode: Text.WordWrap
            }

            Item { width: 1; height: 13 }

            Repeater {
                model: ["Application boundary", "QtBridge adapter", "QML presentation"]

                delegate: Rectangle {
                    required property int index
                    required property string modelData
                    width: parent.width
                    height: 34
                    radius: 5
                    color: index === 0 ? Theme.raised : "transparent"

                    Rectangle {
                        anchors.left: parent.left
                        anchors.leftMargin: 10
                        anchors.verticalCenter: parent.verticalCenter
                        width: 5
                        height: 5
                        radius: 3
                        color: index === 0 ? Theme.accentStrong : Theme.faint
                    }

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 25
                        anchors.verticalCenter: parent.verticalCenter
                        text: modelData
                        color: index === 0 ? Theme.foreground : Theme.muted
                        font.family: Theme.uiFont
                        font.pixelSize: 12
                    }
                }
            }
        }
    }

    Item {
        anchors.left: sidebar.right
        anchors.right: parent.right
        anchors.top: header.bottom
        anchors.bottom: parent.bottom

        Column {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 24
            spacing: 18

            Row {
                width: parent.width
                spacing: 10

                Column {
                    width: parent.width - phaseLabel.width - 10
                    spacing: 4

                    Text {
                        text: root.workspaceTitle(root.activeWorkspace)
                        color: Theme.foreground
                        font.family: Theme.uiFont
                        font.pixelSize: 20
                        font.weight: Font.DemiBold
                    }

                    Text {
                        text: root.workspaceDescription(root.activeWorkspace)
                        color: Theme.muted
                        font.family: Theme.uiFont
                        font.pixelSize: 12
                    }
                }

                Text {
                    id: phaseLabel
                    anchors.verticalCenter: parent.verticalCenter
                    text: "QT FOUNDATION"
                    color: Theme.accentStrong
                    font.family: Theme.monoFont
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    font.letterSpacing: 0.8
                }
            }

            Rectangle {
                width: parent.width
                height: 1
                color: Theme.border
            }

            Column {
                width: parent.width
                spacing: 0

                Repeater {
                    model: [
                        { key: "01", title: "Headless Rust core", detail: "Git, Diff, and Codex state remain outside the UI toolkit." },
                        { key: "02", title: "Official QtBridge", detail: "Properties, commands, and queued callbacks form the adapter seam." },
                        { key: "03", title: "Qt Quick scene", detail: "A single themed shell now owns presentation and interaction." }
                    ]

                    delegate: Rectangle {
                        required property var modelData
                        width: parent.width
                        height: 74
                        color: rowPointer.containsMouse ? Theme.hover : "transparent"

                        Behavior on color {
                            ColorAnimation { duration: Theme.transitionDuration }
                        }

                        Rectangle {
                            anchors.bottom: parent.bottom
                            width: parent.width
                            height: 1
                            color: Theme.border
                        }

                        Text {
                            anchors.left: parent.left
                            anchors.verticalCenter: parent.verticalCenter
                            width: 38
                            text: modelData.key
                            color: Theme.faint
                            font.family: Theme.monoFont
                            font.pixelSize: 11
                        }

                        Column {
                            anchors.left: parent.left
                            anchors.leftMargin: 46
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 5

                            Text {
                                text: modelData.title
                                color: Theme.foreground
                                font.family: Theme.uiFont
                                font.pixelSize: 13
                                font.weight: Font.Medium
                            }

                            Text {
                                width: parent.width
                                text: modelData.detail
                                color: Theme.muted
                                elide: Text.ElideRight
                                font.family: Theme.uiFont
                                font.pixelSize: 12
                            }
                        }

                        MouseArea {
                            id: rowPointer
                            anchors.fill: parent
                            hoverEnabled: true
                        }
                    }
                }
            }
        }
    }
}
