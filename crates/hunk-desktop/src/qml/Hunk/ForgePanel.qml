import QtQuick

Item {
    id: root

    required property var backend
    signal authenticationRequested
    signal reviewRequested

    implicitHeight: content.implicitHeight + 28

    Column {
        id: content
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: 14
        spacing: 9

        Row {
            width: parent.width

            Text {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - refreshButton.width
                text: root.backend.forgeAvailable
                    ? root.backend.forgeProviderLabel.toUpperCase()
                    : "CODE REVIEW"
                color: Theme.muted
                font.family: Theme.uiFont
                font.pixelSize: 10
                font.weight: Font.DemiBold
                font.letterSpacing: 1
            }

            ActionButton {
                id: refreshButton
                label: "Refresh"
                compact: true
                enabled: root.backend.gitReady && !root.backend.forgeBusy
                    && !root.backend.forgeLoading
                onClicked: root.backend.refresh_forge_review()
            }
        }

        Text {
            width: parent.width
            visible: root.backend.forgeLoading
            text: "Resolving review remote…"
            color: Theme.muted
            font.family: Theme.uiFont
            font.pixelSize: 11
        }

        Text {
            width: parent.width
            visible: root.backend.forgeReady && !root.backend.forgeAvailable
            text: root.backend.forgeError.length > 0
                ? root.backend.forgeError
                : "No supported GitHub or GitLab remote for this branch."
            color: Theme.faint
            wrapMode: Text.WordWrap
            font.family: Theme.uiFont
            font.pixelSize: 11
        }

        Column {
            width: parent.width
            visible: root.backend.forgeAvailable
            spacing: 8

            Text {
                width: parent.width
                text: root.backend.forgeRepositoryPath + "  ·  " + root.backend.forgeHost
                color: Theme.faint
                elide: Text.ElideMiddle
                font.family: Theme.monoFont
                font.pixelSize: 9
            }

            Row {
                width: parent.width
                spacing: 8

                Column {
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width - authButton.width - 8
                    spacing: 2

                    Text {
                        width: parent.width
                        text: root.backend.forgeAuthenticated
                            ? (root.backend.forgeAccountLabel || "Credential connected")
                            : (root.backend.forgeAuthMode === "device"
                                ? "GitHub sign-in required"
                                : "Personal access token required")
                        color: root.backend.forgeAuthenticated ? Theme.foreground : Theme.muted
                        elide: Text.ElideRight
                        font.family: Theme.uiFont
                        font.pixelSize: 11
                        font.weight: root.backend.forgeAuthenticated ? Font.Medium : Font.Normal
                    }

                    Text {
                        visible: root.backend.forgeStatusMessage.length > 0
                        width: parent.width
                        text: root.backend.forgeStatusMessage
                        color: Theme.faint
                        elide: Text.ElideRight
                        font.family: Theme.uiFont
                        font.pixelSize: 9
                    }
                }

                ActionButton {
                    id: authButton
                    label: root.backend.forgeAuthenticated
                        ? "Connected"
                        : (root.backend.forgeAuthMode === "device" ? "Sign in" : "Add token")
                    compact: true
                    primary: !root.backend.forgeAuthenticated
                    enabled: !root.backend.forgeAuthenticated && !root.backend.forgeBusy
                    onClicked: root.authenticationRequested()
                }
            }

            Rectangle {
                width: parent.width
                height: 1
                color: Theme.border
            }

            Column {
                width: parent.width
                visible: root.backend.forgeReviewExists
                spacing: 5

                Row {
                    width: parent.width

                    Text {
                        width: parent.width - openReviewButton.width
                        text: root.backend.forgeReviewState.toUpperCase()
                            + "  #" + root.backend.forgeReviewNumber
                        color: root.backend.forgeReviewState === "Merged"
                            ? Theme.positive : Theme.accentStrong
                        font.family: Theme.monoFont
                        font.pixelSize: 9
                        font.weight: Font.DemiBold
                        font.letterSpacing: 0.6
                    }

                    ActionButton {
                        id: openReviewButton
                        label: "Open"
                        compact: true
                        onClicked: Qt.openUrlExternally(root.backend.forgeReviewUrl)
                    }
                }

                Text {
                    width: parent.width
                    text: root.backend.forgeReviewTitle
                    color: Theme.foreground
                    wrapMode: Text.WordWrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                    font.family: Theme.uiFont
                    font.pixelSize: 11
                    font.weight: Font.Medium
                }
            }

            Row {
                width: parent.width
                visible: !root.backend.forgeReviewExists

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width - createReviewButton.width
                    text: root.backend.forgeAuthenticated
                        ? "No review found for this branch"
                        : "Connect an account to create a review"
                    color: Theme.faint
                    font.family: Theme.uiFont
                    font.pixelSize: 10
                }

                ActionButton {
                    id: createReviewButton
                    label: "Create " + (root.backend.forgeProviderLabel === "GitLab" ? "MR" : "PR")
                    compact: true
                    primary: true
                    enabled: root.backend.forgeAuthenticated && !root.backend.forgeBusy
                    onClicked: root.reviewRequested()
                }
            }
        }

        Rectangle {
            width: parent.width
            height: root.backend.forgeError.length > 0 && root.backend.forgeAvailable ? errorText.implicitHeight + 14 : 0
            visible: height > 0
            radius: 5
            color: Theme.negativeMuted

            Text {
                id: errorText
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.margins: 7
                text: root.backend.forgeError
                color: Theme.negative
                wrapMode: Text.WordWrap
                font.family: Theme.uiFont
                font.pixelSize: 10
            }
        }

        Rectangle {
            width: parent.width
            height: deviceContent.implicitHeight + 16
            visible: root.backend.forgeDeviceFlowActive
            radius: 5
            color: Theme.accentMuted

            Column {
                id: deviceContent
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                anchors.margins: 8
                spacing: 7

                Text {
                    width: parent.width
                    text: root.backend.forgeDeviceUserCode
                    color: Theme.foreground
                    horizontalAlignment: Text.AlignHCenter
                    font.family: Theme.monoFont
                    font.pixelSize: 18
                    font.weight: Font.Bold
                    font.letterSpacing: 1.4
                }

                Row {
                    anchors.horizontalCenter: parent.horizontalCenter
                    spacing: 7

                    ActionButton {
                        label: "Open GitHub"
                        compact: true
                        primary: true
                        onClicked: Qt.openUrlExternally(root.backend.forgeDeviceVerificationUrl)
                    }

                    ActionButton {
                        label: "Cancel"
                        compact: true
                        onClicked: root.backend.cancel_github_device_flow()
                    }
                }
            }
        }
    }
}
