pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property var backend
    readonly property var workspaceIds: ["diff", "git", "ai"]
    readonly property int workspaceCount: workspaceIds.length
    readonly property string activeWorkspace: backend.activeWorkspace
    readonly property var sidebarItem: sidebarLoader.item
    readonly property var workspaceItem: workspaceLoader.item

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
                width: Math.min(260, implicitWidth)
                text: root.backend.statusMessage
                color: Theme.muted
                elide: Text.ElideRight
                font.family: Theme.uiFont
                font.pixelSize: 11
            }
        }
    }

    Item {
        id: sidebar
        anchors.left: parent.left
        anchors.top: header.bottom
        anchors.bottom: parent.bottom
        width: Theme.sidebarWidth

        Loader {
            id: sidebarLoader
            anchors.fill: parent
            sourceComponent: root.activeWorkspace === "git" ? gitSidebarComponent
                : (root.activeWorkspace === "diff" ? diffSidebarComponent : summarySidebarComponent)
        }

        Rectangle {
            anchors.right: parent.right
            width: 1
            height: parent.height
            color: Theme.border
        }
    }

    Loader {
        id: workspaceLoader
        objectName: "workspaceLoader"
        anchors.left: sidebar.right
        anchors.right: parent.right
        anchors.top: header.bottom
        anchors.bottom: parent.bottom
        sourceComponent: root.activeWorkspace === "git" ? gitWorkspaceComponent
            : (root.activeWorkspace === "diff" ? diffWorkspaceComponent : placeholderWorkspaceComponent)
    }

    Component {
        id: diffSidebarComponent

        DiffSidebar {
            backend: root.backend
        }
    }

    Component {
        id: diffWorkspaceComponent

        DiffWorkspace {
            objectName: "diffWorkspace"
            backend: root.backend
        }
    }

    Component {
        id: gitSidebarComponent

        GitSidebar {
            backend: root.backend
        }
    }

    Component {
        id: gitWorkspaceComponent

        GitWorkspace {
            objectName: "gitWorkspace"
            backend: root.backend
        }
    }

    Component {
        id: summarySidebarComponent

        Rectangle {
            color: Theme.chrome

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
            }
        }
    }

    Component {
        id: placeholderWorkspaceComponent

        Item {
            Column {
                anchors.centerIn: parent
                width: Math.min(460, parent.width - 48)
                spacing: 8

                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: root.workspaceTitle(root.activeWorkspace)
                    color: Theme.foreground
                    font.family: Theme.uiFont
                    font.pixelSize: 20
                    font.weight: Font.DemiBold
                }

                Text {
                    width: parent.width
                    text: "The Qt Codex surface will connect to the retained Rust thread service in the AI migration layer."
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
}
