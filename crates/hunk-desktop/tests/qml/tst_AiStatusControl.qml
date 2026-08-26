pragma ComponentBehavior: Bound

import QtQuick
import QtTest
import Hunk

Item {
    id: root

    width: 760
    height: 520

    QtObject {
        id: backend

        property bool aiReady: true
        property bool aiLoading: false
        property string aiConnectionState: "ready"
        property int aiApprovalRequestCount: 2
        property int aiInputRequestCount: 1
        property string aiAccountSummary: "ChatGPT: test@example.com (Plus)"
        property bool aiAccountConnected: true
        property bool aiLoginPending: false
        property bool aiFiveHourLimitAvailable: true
        property int aiFiveHourLimitRemainingPercent: 72
        property string aiFiveHourLimitResetLabel: "Resets in 2h"
        property bool aiWeeklyLimitAvailable: true
        property int aiWeeklyLimitRemainingPercent: 41
        property string aiWeeklyLimitResetLabel: "Resets in 4d"
        property string lastCommand: ""

        function start_ai_chatgpt_login() {
            lastCommand = "login";
            aiLoginPending = true;
            return true;
        }

        function cancel_ai_chatgpt_login() {
            lastCommand = "cancel";
            aiLoginPending = false;
            return true;
        }

        function logout_ai_account() {
            lastCommand = "logout";
            aiAccountConnected = false;
            return true;
        }
    }

    Component {
        id: controlComponent

        AiStatusControl {
            backend: backend
        }
    }

    TestCase {
        name: "AiStatusControl"
        when: windowShown

        function init() {
            backend.aiReady = true;
            backend.aiLoading = false;
            backend.aiConnectionState = "ready";
            backend.aiApprovalRequestCount = 2;
            backend.aiInputRequestCount = 1;
            backend.aiAccountSummary = "ChatGPT: test@example.com (Plus)";
            backend.aiAccountConnected = true;
            backend.aiLoginPending = false;
            backend.aiFiveHourLimitAvailable = true;
            backend.aiFiveHourLimitRemainingPercent = 72;
            backend.aiFiveHourLimitResetLabel = "Resets in 2h";
            backend.aiWeeklyLimitAvailable = true;
            backend.aiWeeklyLimitRemainingPercent = 41;
            backend.aiWeeklyLimitResetLabel = "Resets in 4d";
            backend.lastCommand = "";
        }

        function openPopup(control) {
            let button = findChild(control, "aiStatusButton");
            let popup = findChild(control, "aiStatusPopup");
            verify(!!button, "Status button exists");
            verify(!!popup, "Status popup exists");
            mouseClick(button);
            tryCompare(popup, "visible", true);
            return popup;
        }

        function test_metricsAndUsageReflectBackend() {
            let control = createTemporaryObject(controlComponent, root);
            verify(!!control, "Control exists");
            let approvals = findChild(control, "aiApprovalMetric");
            let inputs = findChild(control, "aiInputMetric");
            compare(approvals.text, qsTr("Approvals 2"));
            compare(inputs.text, qsTr("Inputs 1"));

            openPopup(control);
            let account = findChild(control, "aiAccountSummary");
            let fiveHour = findChild(control, "aiFiveHourUsage");
            let weekly = findChild(control, "aiWeeklyUsage");
            compare(account.text, "ChatGPT: test@example.com (Plus)");
            compare(fiveHour.remainingPercent, 72);
            compare(fiveHour.resetLabel, "Resets in 2h");
            compare(weekly.remainingPercent, 41);
            compare(weekly.resetLabel, "Resets in 4d");

            backend.aiWeeklyLimitAvailable = false;
            tryCompare(weekly, "available", false);
        }

        function test_accountActionRoutesLoginCancelAndLogout() {
            let control = createTemporaryObject(controlComponent, root);
            verify(!!control, "Control exists");
            openPopup(control);
            let action = findChild(control, "aiAccountAction");
            compare(action.label, qsTr("Logout"));
            mouseClick(action);
            compare(backend.lastCommand, "logout");

            openPopup(control);
            compare(action.label, qsTr("Login"));
            mouseClick(action);
            compare(backend.lastCommand, "login");

            openPopup(control);
            compare(action.label, qsTr("Cancel login"));
            mouseClick(action);
            compare(backend.lastCommand, "cancel");
        }

        function test_metricsCollapseWithoutHidingStatus() {
            let control = createTemporaryObject(controlComponent, root, { "showMetrics": false });
            verify(!!control, "Control exists");
            compare(findChild(control, "aiApprovalMetric").visible, false);
            compare(findChild(control, "aiInputMetric").visible, false);
            compare(findChild(control, "aiStatusButton").visible, true);
        }

        function test_popupClosesFromKeyboard() {
            let control = createTemporaryObject(controlComponent, root);
            verify(!!control, "Control exists");
            let popup = openPopup(control);
            keyClick(Qt.Key_Escape);
            tryCompare(popup, "visible", false);
        }
    }
}
