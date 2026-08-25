pragma ComponentBehavior: Bound

import QtQuick
import QtTest
import Hunk

Item {
    id: root

    width: 640
    height: 120

    FakeUpdater {
        id: fakeUpdates
    }

    Component {
        id: updateControlComponent

        UpdateControl {
            updates: fakeUpdates
        }
    }

    TestCase {
        name: "UpdateControlTests"
        when: windowShown

        function init() {
            fakeUpdates.enabled = true
            fakeUpdates.busy = false
            fakeUpdates.readyToRestart = false
            fakeUpdates.status = "idle"
            fakeUpdates.statusMessage = ""
            fakeUpdates.version = ""
            fakeUpdates.checkCount = 0
        }

        function test_idleActionStartsManualCheck() {
            const control = createTemporaryObject(updateControlComponent, root)
            verify(!!control, "Update control exists")
            const action = findChild(control, "updateAction")
            verify(!!action, "Update action exists")

            compare(action.label, "Updates")
            mouseClick(action, action.width / 2, action.height / 2)
            compare(fakeUpdates.checkCount, 1)
        }

        function test_busyStateDisablesActionAndShowsStatus() {
            fakeUpdates.busy = true
            fakeUpdates.status = "downloading"
            fakeUpdates.statusMessage = "Downloading Hunk 0.0.12…"
            const control = createTemporaryObject(updateControlComponent, root)
            const action = findChild(control, "updateAction")
            const status = findChild(control, "updateStatus")

            compare(action.enabled, false)
            compare(action.label, "Updating…")
            compare(status.visible, true)
            compare(status.text, fakeUpdates.statusMessage)
        }

        function test_readyStateRequestsConfirmedRestart() {
            fakeUpdates.readyToRestart = true
            fakeUpdates.status = "ready"
            fakeUpdates.version = "0.0.12"
            const control = createTemporaryObject(updateControlComponent, root)
            const action = findChild(control, "updateAction")
            const restartSpy = signalSpy.createObject(root, {
                target: control,
                signalName: "restartRequested"
            })

            compare(action.label, "Restart to update")
            mouseClick(action, action.width / 2, action.height / 2)
            compare(restartSpy.count, 1)
        }
    }

    Component {
        id: signalSpy
        SignalSpy {}
    }
}
