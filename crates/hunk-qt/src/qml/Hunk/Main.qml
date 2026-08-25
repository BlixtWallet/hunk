import QtQuick
import QtQuick.Window
import hunk_qt

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
        Component.onCompleted: backend.bootstrap()
    }

    Shell {
        anchors.fill: parent
        backend: backend
    }
}
