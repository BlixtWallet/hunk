pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property string blockKind
    required property string plainText
    required property string markup
    required property string language
    required property string copyText
    required property int headingLevel
    readonly property Item loadedContent: contentLoader.item as Item
    signal copyRequested(string text)

    implicitHeight: contentLoader.status === Loader.Ready && loadedContent
        ? loadedContent.implicitHeight : 0

    Loader {
        id: contentLoader
        anchors.fill: parent
        sourceComponent: root.blockKind === "code" ? codeComponent
            : (root.blockKind === "rule" ? ruleComponent : textComponent)
    }

    Component {
        id: textComponent

        Item {
            implicitWidth: root.width
            implicitHeight: markdownText.contentHeight

            Rectangle {
                visible: root.blockKind === "quote"
                width: 2
                radius: 1
                color: Theme.borderStrong
                anchors {
                    left: parent.left
                    top: parent.top
                    bottom: parent.bottom
                }
            }

            TextEdit {
                id: markdownText
                objectName: "aiMarkdownText"
                x: root.blockKind === "quote" ? 12 : 0
                width: Math.max(0, parent.width - x)
                height: contentHeight
                text: root.renderedMarkup(root.markup)
                textFormat: TextEdit.RichText
                color: Theme.foreground
                selectionColor: Theme.accent
                selectedTextColor: Theme.foreground
                readOnly: true
                selectByMouse: true
                wrapMode: TextEdit.Wrap
                Accessible.name: root.plainText
                font {
                    family: Theme.uiFont
                    pixelSize: root.blockKind === "heading"
                        ? (root.headingLevel <= 2 ? 15 : 14) : 13
                    weight: root.blockKind === "heading" ? Font.DemiBold : Font.Normal
                }
            }
        }
    }

    Component {
        id: codeComponent

        Rectangle {
            objectName: "aiMarkdownCodeBlock"
            implicitWidth: root.width
            implicitHeight: codeColumn.implicitHeight
            radius: 6
            color: Theme.input
            border.width: 1
            border.color: Theme.border

            Column {
                id: codeColumn
                width: parent.width

                Item {
                    width: parent.width
                    height: 30

                    Text {
                        objectName: "aiMarkdownCodeLanguage"
                        text: root.language.length > 0
                            ? root.language.toUpperCase() : qsTr("CODE")
                        textFormat: Text.PlainText
                        color: Theme.faint
                        elide: Text.ElideRight
                        font {
                            family: Theme.monoFont
                            pixelSize: 9
                            weight: Font.DemiBold
                            letterSpacing: 0.7
                        }
                        anchors {
                            left: parent.left
                            leftMargin: 12
                            right: copyButton.left
                            rightMargin: 8
                            verticalCenter: parent.verticalCenter
                        }
                    }

                    ActionButton {
                        id: copyButton
                        objectName: "aiMarkdownCopyButton"
                        label: qsTr("Copy")
                        accessibleName: qsTr("Copy code block")
                        compact: true
                        onClicked: root.copyRequested(root.copyText)
                        anchors {
                            right: parent.right
                            rightMargin: 4
                            verticalCenter: parent.verticalCenter
                        }
                    }

                    Rectangle {
                        height: 1
                        color: Theme.border
                        anchors {
                            left: parent.left
                            right: parent.right
                            bottom: parent.bottom
                        }
                    }
                }

                Item {
                    width: parent.width
                    height: codeText.contentHeight + 20

                    TextEdit {
                        id: codeText
                        objectName: "aiMarkdownCodeText"
                        anchors.fill: parent
                        anchors.margins: 10
                        text: root.renderedMarkup(root.markup)
                        textFormat: TextEdit.RichText
                        color: Theme.foreground
                        selectionColor: Theme.accent
                        selectedTextColor: Theme.foreground
                        readOnly: true
                        selectByMouse: true
                        wrapMode: TextEdit.WrapAnywhere
                        font.family: Theme.monoFont
                        font.pixelSize: 11
                        Accessible.name: qsTr("%1 code block").arg(
                            root.language.length > 0 ? root.language : qsTr("Plain text"))
                    }
                }
            }
        }
    }

    Component {
        id: ruleComponent

        Item {
            implicitWidth: root.width
            implicitHeight: 11

            Rectangle {
                height: 1
                color: Theme.border
                anchors {
                    left: parent.left
                    right: parent.right
                    verticalCenter: parent.verticalCenter
                }
            }
        }
    }

    function renderedMarkup(value) {
        const colors = {
            plain: Theme.foreground,
            link: Theme.accentStrong,
            keyword: Theme.syntaxKeyword,
            string: Theme.syntaxString,
            number: Theme.syntaxNumber,
            comment: Theme.syntaxComment,
            function: Theme.syntaxFunction,
            type: Theme.syntaxType,
            constant: Theme.syntaxConstant,
            variable: Theme.syntaxVariable,
            operator: Theme.syntaxOperator
        };
        return value.replace(/@([a-z]+)@/g, function(match, token) {
            const color = colors[token];
            return color === undefined ? match : String(color);
        });
    }
}
