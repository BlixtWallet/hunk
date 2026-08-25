import QtQuick
import QtTest
import Hunk 1.0

TestCase {
    name: "HunkShell"

    property bool snapshotReady: false
    property bool snapshotSaved: false

    QtObject {
        id: fakeBackend

        property string activeWorkspace: "diff"
        property bool ready: true
        property string statusMessage: "Test backend ready"
        property string lastRequestedWorkspace: ""

        function select_workspace(workspace) {
            lastRequestedWorkspace = workspace
            activeWorkspace = workspace
        }
    }

    Shell {
        id: shell
        width: 1200
        height: 760
        backend: fakeBackend
    }

    function init() {
        fakeBackend.activeWorkspace = "diff"
        fakeBackend.lastRequestedWorkspace = ""
        snapshotReady = false
        snapshotSaved = false
    }

    function test_retainedWorkspaceContract() {
        compare(shell.workspaceCount, 3)
        compare(shell.workspaceIds, ["diff", "git", "ai"])
    }

    function test_workspaceActivationUsesBackendCommand() {
        shell.activateWorkspace("git")
        compare(fakeBackend.lastRequestedWorkspace, "git")
        compare(shell.activeWorkspace, "git")
    }

    function test_shellRendersAtDesktopSize() {
        shell.grabToImage(function(result) {
            snapshotSaved = result.saveToFile("target/hunk-qt-shell.png")
            snapshotReady = true
        })

        tryVerify(() => snapshotReady)
        verify(snapshotSaved)
    }
}
