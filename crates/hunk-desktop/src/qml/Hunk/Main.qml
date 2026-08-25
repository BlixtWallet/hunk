import QtQuick
import hunk_desktop
import Hunk.Native

Window {
    id: window

    width: 1360
    height: 840
    minimumWidth: 920
    minimumHeight: 600
    visible: true
    title: "Hunk"
    color: Theme.canvas

    Backend {
        id: backend
        Component.onCompleted: {
            backend.bootstrap()
            backend.updates.bootstrap()
        }
        Component.onDestruction: {
            backend.browser.shutdown()
            backend.updates.shutdown()
        }
    }

    Connections {
        target: backend.updates
        function onQuitRequested() { Qt.quit() }
    }

    Component {
        id: browserSurfaceComponent

        BrowserFrameItem {}
    }

    Shell {
        anchors.fill: parent
        backend: backend
        browserSurfaceComponent: browserSurfaceComponent
    }
}
