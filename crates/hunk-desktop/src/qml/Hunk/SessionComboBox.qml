pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Basic

ComboBox {
    id: root

    property string accessibleName: ""

    implicitWidth: 220
    implicitHeight: 30
    textRole: "label"
    activeFocusOnTab: enabled
    font.family: Theme.uiFont
    font.pixelSize: 11
    delegate: delegateComponent

    indicator: Text {
        x: root.width - width - 9
        y: (root.height - height) / 2 - 1
        text: "⌄"
        color: Theme.muted
        font.family: Theme.uiFont
        font.pixelSize: 12
    }

    contentItem: Text {
        leftPadding: 10
        rightPadding: 24
        text: root.displayText
        textFormat: Text.PlainText
        color: Theme.foreground
        elide: Text.ElideRight
        verticalAlignment: Text.AlignVCenter
        font: root.font
    }

    background: Rectangle {
        radius: 5
        color: Theme.input
        border.width: 1
        border.color: root.activeFocus ? Theme.accentStrong : Theme.border
    }

    popup: Popup {
        y: root.height + 4
        width: root.width
        implicitHeight: Math.min(contentItem.implicitHeight + 2, 264)
        padding: 1

        contentItem: ListView {
            clip: true
            implicitHeight: contentHeight
            model: root.popup.visible ? root.delegateModel : null
            currentIndex: root.highlightedIndex
            boundsBehavior: Flickable.StopAtBounds
            ScrollIndicator.vertical: ScrollIndicator {}
        }

        background: Rectangle {
            radius: 5
            color: Theme.raised
            border.width: 1
            border.color: Theme.borderStrong
        }
    }

    Accessible.name: accessibleName

    Component {
        id: delegateComponent

        ItemDelegate {
            id: delegateRoot
            required property var model
            required property int index

            width: ListView.view ? ListView.view.width : root.width
            height: 32
            text: model.label
            highlighted: root.highlightedIndex === index
            font.family: Theme.uiFont
            font.pixelSize: 11
            palette.text: Theme.foreground
            palette.highlightedText: Theme.foreground

            contentItem: Text {
                text: delegateRoot.text
                textFormat: Text.PlainText
                color: Theme.foreground
                elide: Text.ElideRight
                verticalAlignment: Text.AlignVCenter
                font: delegateRoot.font
            }

            background: Rectangle {
                color: delegateRoot.highlighted ? Theme.selected : Theme.raised
            }
        }
    }
}
