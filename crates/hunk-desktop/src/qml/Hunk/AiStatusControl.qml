pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic

FocusScope {
    id: root

    required property var backend
    property bool showMetrics: true
    property bool popupAbove: false
    readonly property string connectionLabel: backend.aiConnectionState === "ready" ? qsTr("Connected")
        : (backend.aiConnectionState === "connecting" ? qsTr("Connecting")
            : (backend.aiConnectionState === "waiting" ? qsTr("Waiting")
                : (backend.aiConnectionState === "failed" ? qsTr("Failed") : qsTr("Disconnected"))))
    readonly property color connectionColor: backend.aiConnectionState === "ready" ? Theme.positive
        : (backend.aiConnectionState === "failed" ? Theme.negative : Theme.warning)
    readonly property string accountActionLabel: backend.aiLoginPending ? qsTr("Cancel login")
        : (backend.aiAccountConnected ? qsTr("Logout") : qsTr("Login"))

    implicitWidth: headerRow.implicitWidth
    implicitHeight: headerRow.implicitHeight

    Row {
        id: headerRow
        spacing: 8

        Text {
            objectName: "aiApprovalMetric"
            anchors.verticalCenter: parent.verticalCenter
            visible: root.showMetrics
            text: qsTr("Approvals %1").arg(root.backend.aiApprovalRequestCount)
            textFormat: Text.PlainText
            color: root.backend.aiApprovalRequestCount > 0 ? Theme.warning : Theme.muted
            font.family: Theme.monoFont
            font.pixelSize: 9
        }

        Text {
            objectName: "aiInputMetric"
            anchors.verticalCenter: parent.verticalCenter
            visible: root.showMetrics
            text: qsTr("Inputs %1").arg(root.backend.aiInputRequestCount)
            textFormat: Text.PlainText
            color: root.backend.aiInputRequestCount > 0 ? Theme.warning : Theme.muted
            font.family: Theme.monoFont
            font.pixelSize: 9
        }

        ActionButton {
            objectName: "aiStatusButton"
            label: qsTr("Status")
            accessibleName: qsTr("Codex status: %1").arg(root.connectionLabel)
            compact: true
            onClicked: {
                if (statusPopup.opened)
                    statusPopup.close();
                else
                    statusPopup.open();
            }
        }

        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: 7
            height: 7
            radius: 4
            color: root.connectionColor
            Accessible.ignored: true
        }

        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: root.connectionLabel.toUpperCase()
            textFormat: Text.PlainText
            color: Theme.muted
            font {
                family: Theme.monoFont
                pixelSize: 9
                letterSpacing: 0.5
            }
        }
    }

    Popup {
        id: statusPopup
        objectName: "aiStatusPopup"
        x: root.width - width
        y: root.popupAbove ? -height - 7 : root.height + 7
        width: 360
        height: contentItem.implicitHeight + topPadding + bottomPadding
        padding: 14
        focus: true
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
        background: Rectangle {
            radius: Theme.radius
            color: Theme.chrome
            border.width: 1
            border.color: Theme.borderStrong
        }

        contentItem: Column {
            width: statusPopup.availableWidth
            spacing: 11

            Row {
                width: parent.width
                spacing: 8

                Text {
                    width: parent.width - connectionSummary.width - closeButton.width - parent.spacing * 2
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Codex status")
                    textFormat: Text.PlainText
                    color: Theme.foreground
                    font {
                        family: Theme.uiFont
                        pixelSize: 13
                        weight: Font.DemiBold
                    }
                }

                Text {
                    id: connectionSummary
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.connectionLabel.toUpperCase()
                    textFormat: Text.PlainText
                    color: root.connectionColor
                    font {
                        family: Theme.monoFont
                        pixelSize: 9
                        letterSpacing: 0.5
                    }
                }

                ActionButton {
                    id: closeButton
                    objectName: "aiStatusCloseButton"
                    label: qsTr("Close")
                    compact: true
                    onClicked: statusPopup.close()
                }
            }

            Text {
                objectName: "aiAccountSummary"
                width: parent.width
                text: root.backend.aiAccountSummary
                textFormat: Text.PlainText
                color: Theme.muted
                wrapMode: Text.WordWrap
                font.family: Theme.uiFont
                font.pixelSize: 11
            }

            Rectangle {
                width: parent.width
                height: 1
                color: Theme.border
                Accessible.ignored: true
            }

            AiUsageRow {
                objectName: "aiFiveHourUsage"
                width: parent.width
                label: qsTr("5h limit")
                available: root.backend.aiFiveHourLimitAvailable
                remainingPercent: root.backend.aiFiveHourLimitRemainingPercent
                resetLabel: root.backend.aiFiveHourLimitResetLabel
            }

            AiUsageRow {
                objectName: "aiWeeklyUsage"
                width: parent.width
                label: qsTr("7d limit")
                available: root.backend.aiWeeklyLimitAvailable
                remainingPercent: root.backend.aiWeeklyLimitRemainingPercent
                resetLabel: root.backend.aiWeeklyLimitResetLabel
            }

            ActionButton {
                objectName: "aiAccountAction"
                label: root.accountActionLabel
                accessibleName: root.accountActionLabel
                danger: root.backend.aiAccountConnected && !root.backend.aiLoginPending
                enabled: root.backend.aiReady && !root.backend.aiLoading
                onClicked: {
                    let accepted = root.backend.aiLoginPending
                        ? root.backend.cancel_ai_chatgpt_login()
                        : (root.backend.aiAccountConnected
                            ? root.backend.logout_ai_account()
                            : root.backend.start_ai_chatgpt_login());
                    if (accepted)
                        statusPopup.close();
                }
            }
        }
    }
}
