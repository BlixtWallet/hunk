pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic

FocusScope {
    id: root

    required property var backend
    readonly property string summary: backend.aiSelectedModelLabel + " · " + backend.aiSelectedEffortLabel

    implicitWidth: actionRow.implicitWidth
    implicitHeight: actionRow.implicitHeight

    ListModel {
        id: modeOptions
        ListElement {
            label: qsTr("Code")
            value: "code"
        }
        ListElement {
            label: qsTr("Plan")
            value: "plan"
        }
    }

    ListModel {
        id: approvalOptions
        ListElement {
            label: qsTr("Ask for approvals")
            enabledValue: false
        }
        ListElement {
            label: qsTr("Full access")
            enabledValue: true
        }
    }

    Row {
        id: actionRow
        spacing: 6

        ActionButton {
            id: settingsButton
            objectName: "aiSessionSettingsButton"
            label: root.summary
            accessibleName: qsTr("Configure Codex session: %1").arg(root.summary)
            compact: true
            maximumWidth: 228
            labelElide: Text.ElideMiddle
            enabled: root.backend.aiReady
            onClicked: settingsPopup.open()
        }

        ActionButton {
            id: contextButton
            objectName: "aiContextUsageButton"
            label: root.backend.aiContextPercentUsed + qsTr("% context")
            accessibleName: qsTr("Context window: %1 percent used").arg(root.backend.aiContextPercentUsed)
            compact: true
            visible: root.backend.aiContextAvailable
            enabled: root.backend.aiReady
            onClicked: settingsPopup.open()
        }
    }

    Popup {
        id: settingsPopup
        objectName: "aiSessionSettingsPopup"
        x: root.width - width
        y: root.height + 7
        width: 374
        padding: 14
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        contentItem: settingsLoader
        background: Rectangle {
            radius: Theme.radius
            color: Theme.chrome
            border.width: 1
            border.color: Theme.borderStrong
        }

        Loader {
            id: settingsLoader
            active: settingsPopup.visible
            sourceComponent: settingsContentComponent
        }

        Component {
            id: settingsContentComponent

            Column {
                width: settingsPopup.availableWidth
                spacing: 10

                Row {
                    width: parent.width
                    spacing: 8

                    Text {
                        width: parent.width - lockLabel.width - parent.spacing
                        text: qsTr("Codex session")
                        textFormat: Text.PlainText
                        color: Theme.foreground
                        font {
                            family: Theme.uiFont
                            pixelSize: 13
                            weight: Font.DemiBold
                        }
                    }

                    Text {
                        id: lockLabel
                        text: root.backend.aiSessionControlsLocked ? qsTr("LOCKED") : qsTr("READY")
                        textFormat: Text.PlainText
                        color: root.backend.aiSessionControlsLocked ? Theme.warning : Theme.faint
                        font {
                            family: Theme.monoFont
                            pixelSize: 9
                            letterSpacing: 0.5
                        }
                    }
                }

                Text {
                    width: parent.width
                    visible: root.backend.aiSessionControlsLocked
                    text: qsTr("Session settings unlock when the active turn finishes.")
                    textFormat: Text.PlainText
                    color: Theme.muted
                    wrapMode: Text.WordWrap
                    font.family: Theme.uiFont
                    font.pixelSize: 10
                }

                Row {
                    spacing: 12
                    Text {
                        width: 104
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Model")
                        color: Theme.muted
                        font.family: Theme.uiFont
                        font.pixelSize: 11
                    }
                    SessionComboBox {
                        id: modelSelector
                        objectName: "aiSessionModelSelector"
                        model: root.backend.aiModels
                        accessibleName: qsTr("Codex model")
                        enabled: !root.backend.aiSessionControlsLocked
                        onActivated: index => {
                            if (!root.backend.select_ai_model(index))
                                modelSelector.currentIndex = Qt.binding(() => root.backend.aiSelectedModelIndex);
                        }
                        Binding on currentIndex {
                            value: root.backend.aiSelectedModelIndex
                        }
                    }
                }

                Row {
                    spacing: 12
                    Text {
                        width: 104
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Reasoning")
                        color: Theme.muted
                        font.family: Theme.uiFont
                        font.pixelSize: 11
                    }
                    SessionComboBox {
                        id: effortSelector
                        objectName: "aiSessionEffortSelector"
                        model: root.backend.aiEfforts
                        accessibleName: qsTr("Reasoning effort")
                        enabled: !root.backend.aiSessionControlsLocked && root.backend.aiSelectedModelIndex > 0 && root.backend.aiEffortOptionCount > 1
                        onActivated: index => {
                            if (!root.backend.select_ai_effort(index))
                                effortSelector.currentIndex = Qt.binding(() => root.backend.aiSelectedEffortIndex);
                        }
                        Binding on currentIndex {
                            value: root.backend.aiSelectedEffortIndex
                        }
                    }
                }

                Row {
                    spacing: 12
                    Text {
                        width: 104
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Mode")
                        color: Theme.muted
                        font.family: Theme.uiFont
                        font.pixelSize: 11
                    }
                    SessionComboBox {
                        id: modeSelector
                        objectName: "aiSessionModeSelector"
                        model: modeOptions
                        accessibleName: qsTr("Codex collaboration mode")
                        enabled: !root.backend.aiSessionControlsLocked
                        onActivated: index => {
                            if (!root.backend.select_ai_collaboration_mode(modeOptions.get(index).value)) {
                                modeSelector.currentIndex = Qt.binding(() => root.backend.aiSelectedCollaborationMode === "plan" ? 1 : 0);
                            }
                        }
                        Binding on currentIndex {
                            value: root.backend.aiSelectedCollaborationMode === "plan" ? 1 : 0
                        }
                    }
                }

                Row {
                    spacing: 12
                    Text {
                        width: 104
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Service tier")
                        color: Theme.muted
                        font.family: Theme.uiFont
                        font.pixelSize: 11
                    }
                    SessionComboBox {
                        id: serviceSelector
                        objectName: "aiSessionServiceSelector"
                        model: root.backend.aiServiceTiers
                        accessibleName: qsTr("Codex service tier")
                        enabled: !root.backend.aiSessionControlsLocked
                        onActivated: index => {
                            if (!root.backend.select_ai_service_tier(index))
                                serviceSelector.currentIndex = Qt.binding(() => root.backend.aiSelectedServiceTierIndex);
                        }
                        Binding on currentIndex {
                            value: root.backend.aiSelectedServiceTierIndex
                        }
                    }
                }

                Row {
                    spacing: 12
                    Text {
                        width: 104
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Approval policy")
                        color: Theme.muted
                        font.family: Theme.uiFont
                        font.pixelSize: 11
                    }
                    SessionComboBox {
                        id: approvalSelector
                        objectName: "aiSessionApprovalSelector"
                        model: approvalOptions
                        accessibleName: qsTr("Codex approval policy")
                        enabled: !root.backend.aiSessionControlsLocked
                        onActivated: index => {
                            if (!root.backend.set_ai_mad_max_mode(approvalOptions.get(index).enabledValue)) {
                                approvalSelector.currentIndex = Qt.binding(() => root.backend.aiMadMaxMode ? 1 : 0);
                            }
                        }
                        Binding on currentIndex {
                            value: root.backend.aiMadMaxMode ? 1 : 0
                        }
                    }
                }

                Rectangle {
                    width: parent.width
                    height: 1
                    color: Theme.border
                }

                Column {
                    width: parent.width
                    spacing: 6

                    Row {
                        width: parent.width
                        Text {
                            width: parent.width / 2
                            text: qsTr("Context window")
                            color: Theme.foreground
                            font {
                                family: Theme.uiFont
                                pixelSize: 11
                                weight: Font.DemiBold
                            }
                        }
                        Text {
                            objectName: "aiContextPercentLabel"
                            width: parent.width / 2
                            text: root.backend.aiContextAvailable ? qsTr("%1% used · %2% left").arg(root.backend.aiContextPercentUsed).arg(root.backend.aiContextPercentLeft) : qsTr("No usage yet")
                            color: Theme.muted
                            horizontalAlignment: Text.AlignRight
                            font.family: Theme.uiFont
                            font.pixelSize: 10
                        }
                    }

                    Text {
                        objectName: "aiContextTokenSummary"
                        width: parent.width
                        text: root.backend.aiContextTokenSummary
                        textFormat: Text.PlainText
                        color: Theme.faint
                        font.family: Theme.monoFont
                        font.pixelSize: 9
                    }

                    Grid {
                        objectName: "aiContextDetails"
                        visible: root.backend.aiContextAvailable
                        columns: 4
                        columnSpacing: 10
                        rowSpacing: 4

                        Text {
                            text: qsTr("Input")
                            color: Theme.faint
                            font.family: Theme.uiFont
                            font.pixelSize: 9
                        }
                        Text {
                            text: root.backend.aiContextInputTokens
                            color: Theme.muted
                            font.family: Theme.monoFont
                            font.pixelSize: 9
                        }
                        Text {
                            text: qsTr("Cached")
                            color: Theme.faint
                            font.family: Theme.uiFont
                            font.pixelSize: 9
                        }
                        Text {
                            text: root.backend.aiContextCachedInputTokens
                            color: Theme.muted
                            font.family: Theme.monoFont
                            font.pixelSize: 9
                        }
                        Text {
                            text: qsTr("Output")
                            color: Theme.faint
                            font.family: Theme.uiFont
                            font.pixelSize: 9
                        }
                        Text {
                            text: root.backend.aiContextOutputTokens
                            color: Theme.muted
                            font.family: Theme.monoFont
                            font.pixelSize: 9
                        }
                        Text {
                            text: qsTr("Reasoning")
                            color: Theme.faint
                            font.family: Theme.uiFont
                            font.pixelSize: 9
                        }
                        Text {
                            text: root.backend.aiContextReasoningTokens
                            color: Theme.muted
                            font.family: Theme.monoFont
                            font.pixelSize: 9
                        }
                        Text {
                            text: qsTr("Billable")
                            color: Theme.faint
                            font.family: Theme.uiFont
                            font.pixelSize: 9
                        }
                        Text {
                            text: root.backend.aiContextBillableTokens
                            color: Theme.muted
                            font.family: Theme.monoFont
                            font.pixelSize: 9
                        }
                    }
                }
            }
        }
    }
}
