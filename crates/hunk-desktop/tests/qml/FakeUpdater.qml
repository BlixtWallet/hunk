import QtQuick

QtObject {
    property bool enabled: true
    property bool busy: false
    property bool readyToRestart: false
    property string status: "idle"
    property string statusMessage: ""
    property string version: ""
    property int checkCount: 0
    property int pollCount: 0

    function check_for_updates() {
        checkCount += 1
        return true
    }

    function poll() {
        pollCount += 1
        return true
    }
}
