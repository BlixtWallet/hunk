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
    property var aiDraftStore: ({})
    property string aiDraftWorkspaceRoot: ""

    function activateWorkspace(workspace) {
        backend.select_workspace(workspace)
    }

    function syncAiDraftWorkspace() {
        if (aiDraftWorkspaceRoot === backend.aiWorkspaceRoot)
            return
        aiDraftWorkspaceRoot = backend.aiWorkspaceRoot
        aiDraftStore = ({})
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
                : (root.activeWorkspace === "diff" ? diffSidebarComponent : aiSidebarComponent)
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
            : (root.activeWorkspace === "diff" ? diffWorkspaceComponent : aiWorkspaceComponent)
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
        id: aiSidebarComponent

        AiSidebar {
            backend: root.backend
        }
    }

    Component {
        id: aiWorkspaceComponent

        AiWorkspace {
            objectName: "aiWorkspace"
            backend: root.backend
            draftStore: root.aiDraftStore
        }
    }

    Connections {
        target: root.backend

        function onAiStateChanged() {
            root.syncAiDraftWorkspace()
        }
    }

    Component.onCompleted: syncAiDraftWorkspace()
}
