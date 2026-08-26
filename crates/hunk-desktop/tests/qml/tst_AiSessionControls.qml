pragma ComponentBehavior: Bound

import QtQuick
import QtTest
import Hunk

Item {
    id: root
    property alias backendObject: backend

    width: 640
    height: 520

    ListModel {
        id: models
        ListElement {
            value: ""
            label: qsTr("Server default")
            description: ""
        }
        ListElement {
            value: "gpt-test"
            label: qsTr("GPT Test")
            description: ""
        }
    }

    ListModel {
        id: efforts
        ListElement {
            value: ""
            label: qsTr("Model default")
            description: ""
        }
        ListElement {
            value: "low"
            label: qsTr("Low")
            description: ""
        }
        ListElement {
            value: "high"
            label: qsTr("High")
            description: ""
        }
    }

    ListModel {
        id: serviceTiers
        ListElement {
            value: "standard"
            label: qsTr("Standard")
            description: ""
        }
        ListElement {
            value: "fast"
            label: qsTr("Fast")
            description: ""
        }
        ListElement {
            value: "flex"
            label: qsTr("Flex")
            description: ""
        }
    }

    QtObject {
        id: backend

        property bool aiReady: true
        property QtObject aiModels: models
        property QtObject aiEfforts: efforts
        property QtObject aiServiceTiers: serviceTiers
        property int aiSelectedModelIndex: 1
        property int aiSelectedEffortIndex: 0
        property int aiSelectedServiceTierIndex: 0
        property string aiSelectedModelLabel: qsTr("GPT Test")
        property string aiSelectedEffortLabel: qsTr("Model default")
        property string aiSelectedCollaborationMode: "code"
        property string aiSelectedCollaborationLabel: qsTr("Code")
        property string aiSelectedServiceTierLabel: qsTr("Standard")
        property bool aiMadMaxMode: false
        property string aiApprovalPolicyLabel: qsTr("Ask for approvals")
        property int aiEffortOptionCount: 3
        property bool aiSessionControlsLocked: false
        property bool aiContextAvailable: true
        property int aiContextPercentUsed: 50
        property int aiContextPercentLeft: 50
        property string aiContextTokenSummary: qsTr("72k / 132k tokens")
        property string aiContextInputTokens: qsTr("30,000")
        property string aiContextCachedInputTokens: qsTr("12,000")
        property string aiContextOutputTokens: qsTr("8,000")
        property string aiContextReasoningTokens: qsTr("3,000")
        property string aiContextBillableTokens: qsTr("38,000")
        property string lastCommand: ""
        property int lastIndex: -1
        property string lastValue: ""
        property bool rejectSelections: false

        function select_ai_model(index) {
            if (rejectSelections)
                return false;
            lastCommand = "model";
            lastIndex = index;
            aiSelectedModelIndex = index;
            aiSelectedModelLabel = models.get(index).label;
            return true;
        }

        function select_ai_effort(index) {
            if (rejectSelections)
                return false;
            lastCommand = "effort";
            lastIndex = index;
            aiSelectedEffortIndex = index;
            aiSelectedEffortLabel = efforts.get(index).label;
            return true;
        }

        function select_ai_collaboration_mode(value) {
            if (rejectSelections)
                return false;
            lastCommand = "mode";
            lastValue = value;
            aiSelectedCollaborationMode = value;
            aiSelectedCollaborationLabel = value === "plan" ? qsTr("Plan") : qsTr("Code");
            return true;
        }

        function select_ai_service_tier(index) {
            if (rejectSelections)
                return false;
            lastCommand = "service";
            lastIndex = index;
            aiSelectedServiceTierIndex = index;
            aiSelectedServiceTierLabel = serviceTiers.get(index).label;
            return true;
        }

        function set_ai_mad_max_mode(enabled) {
            if (rejectSelections)
                return false;
            lastCommand = "approval";
            aiMadMaxMode = enabled;
            return true;
        }
    }

    Component {
        id: controlsComponent
        AiSessionControls {
            width: implicitWidth
            height: implicitHeight
            backend: root.backendObject
        }
    }

    TestCase {
        name: "AiSessionControls"
        when: windowShown

        function init() {
            backend.aiSelectedModelIndex = 1;
            backend.aiSelectedEffortIndex = 0;
            backend.aiSelectedServiceTierIndex = 0;
            backend.aiSelectedModelLabel = qsTr("GPT Test");
            backend.aiSelectedEffortLabel = qsTr("Model default");
            backend.aiSelectedCollaborationMode = "code";
            backend.aiSelectedCollaborationLabel = qsTr("Code");
            backend.aiSelectedServiceTierLabel = qsTr("Standard");
            backend.aiMadMaxMode = false;
            backend.aiSessionControlsLocked = false;
            backend.aiContextAvailable = true;
            backend.lastCommand = "";
            backend.lastIndex = -1;
            backend.lastValue = "";
            backend.rejectSelections = false;
        }

        function openSettings(controls) {
            let settingsButton = findChild(controls, "aiSessionSettingsButton");
            verify(!!settingsButton, "Object exists");
            let settingsPopup = findChild(controls, "aiSessionSettingsPopup");
            verify(!!settingsPopup, "Object exists");
            mouseClick(settingsButton);
            tryCompare(settingsPopup, "visible", true);
            return settingsPopup;
        }

        function chooseNext(controls, objectName) {
            let selector = findChild(controls, objectName);
            verify(!!selector, "Object exists");
            selector.activated(selector.currentIndex + 1);
            return selector;
        }

        function test_popupOpensAndShowsContextUsage() {
            let controls = createTemporaryObject(controlsComponent, root);
            verify(!!controls, "Component exists");
            let popup = openSettings(controls);
            tryCompare(popup, "visible", true);
            verify(popup.height > 240, "Popup content has a visible height");
            compare(controls.summary, qsTr("GPT Test · Model default"));
            let percentLabel = findChild(controls, "aiContextPercentLabel");
            let tokenSummary = findChild(controls, "aiContextTokenSummary");
            let details = findChild(controls, "aiContextDetails");
            verify(!!percentLabel, "Context percentage exists");
            verify(!!tokenSummary, "Context token summary exists");
            verify(!!details, "Context details exist");
            compare(percentLabel.text, qsTr("50% used · 50% left"));
            compare(tokenSummary.text, qsTr("72k / 132k tokens"));

            backend.aiContextAvailable = false;
            tryCompare(percentLabel, "text", qsTr("No usage yet"));
            tryCompare(details, "visible", false);
        }

        function test_modelSelectionRoutesToBackend() {
            backend.aiSelectedModelIndex = 0;
            let controls = createTemporaryObject(controlsComponent, root);
            verify(!!controls, "Component exists");
            openSettings(controls);
            chooseNext(controls, "aiSessionModelSelector");
            tryCompare(backend, "lastCommand", "model");
            compare(backend.lastIndex, 1);
        }

        function test_effortSelectionRoutesToBackend() {
            let controls = createTemporaryObject(controlsComponent, root);
            verify(!!controls, "Component exists");
            openSettings(controls);
            chooseNext(controls, "aiSessionEffortSelector");
            tryCompare(backend, "lastCommand", "effort");
            compare(backend.lastIndex, 1);
        }

        function test_modeSelectionRoutesToBackend() {
            let controls = createTemporaryObject(controlsComponent, root);
            verify(!!controls, "Component exists");
            openSettings(controls);
            chooseNext(controls, "aiSessionModeSelector");
            tryCompare(backend, "lastCommand", "mode");
            compare(backend.lastValue, "plan");
        }

        function test_serviceSelectionRoutesToBackend() {
            let controls = createTemporaryObject(controlsComponent, root);
            verify(!!controls, "Component exists");
            openSettings(controls);
            chooseNext(controls, "aiSessionServiceSelector");
            tryCompare(backend, "lastCommand", "service");
            compare(backend.lastIndex, 1);
        }

        function test_approvalSelectionRoutesToBackend() {
            let controls = createTemporaryObject(controlsComponent, root);
            verify(!!controls, "Component exists");
            openSettings(controls);
            chooseNext(controls, "aiSessionApprovalSelector");
            tryCompare(backend, "lastCommand", "approval");
            compare(backend.aiMadMaxMode, true);
        }

        function test_runningTurnLocksEveryMutableSelector() {
            backend.aiSessionControlsLocked = true;
            let controls = createTemporaryObject(controlsComponent, root);
            verify(!!controls, "Component exists");
            openSettings(controls);
            const objectNames = ["aiSessionModelSelector", "aiSessionEffortSelector", "aiSessionModeSelector", "aiSessionServiceSelector", "aiSessionApprovalSelector"];
            for (const objectName of objectNames) {
                let selector = findChild(controls, objectName);
                verify(!!selector, "Object exists");
                compare(selector.enabled, false);
            }
        }

        function test_rejectedSelectionRestoresAuthoritativeIndex() {
            backend.aiSelectedModelIndex = 0;
            backend.rejectSelections = true;
            let controls = createTemporaryObject(controlsComponent, root);
            verify(!!controls, "Component exists");
            openSettings(controls);
            let selector = chooseNext(controls, "aiSessionModelSelector");
            tryCompare(selector, "currentIndex", 0);
            compare(backend.lastCommand, "");
        }
    }
}
