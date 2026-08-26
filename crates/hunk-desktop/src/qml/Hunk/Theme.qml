pragma Singleton

import QtQuick

QtObject {
    readonly property color transparent: "transparent"
    readonly property color canvas: "#0f1720"
    readonly property color chrome: "#172431"
    readonly property color raised: "#203140"
    readonly property color input: "#18232d"
    readonly property color hover: "#263a4b"
    readonly property color selected: "#294766"
    readonly property color border: "#2b3e4f"
    readonly property color borderStrong: "#486174"
    readonly property color accent: "#4e91df"
    readonly property color accentStrong: "#79b5f5"
    readonly property color accentMuted: "#1f3e60"
    readonly property color foreground: "#f1f5f8"
    readonly property color muted: "#b8c4cd"
    readonly property color faint: "#a8b5c0"
    readonly property color positive: "#5ad39a"
    readonly property color positiveMuted: "#193d34"
    readonly property color negative: "#f17484"
    readonly property color negativeMuted: "#4b2732"
    readonly property color warning: "#e4ad63"
    readonly property color overlay: "#b00b1118"
    readonly property color syntaxKeyword: "#c586c0"
    readonly property color syntaxString: "#ce9178"
    readonly property color syntaxNumber: "#b5cea8"
    readonly property color syntaxComment: "#6a9955"
    readonly property color syntaxFunction: "#dcdcaa"
    readonly property color syntaxType: "#4ec9b0"
    readonly property color syntaxConstant: "#4fc1ff"
    readonly property color syntaxVariable: "#9cdcfe"
    readonly property color syntaxOperator: "#d4d4d4"
    readonly property color terminalBackground: input
    readonly property color terminalForeground: foreground
    readonly property color terminalCursor: accentStrong
    readonly property color terminalCursorMuted: "#8f7d9eff"
    readonly property color terminalBlack: "#1b1b1b"
    readonly property color terminalRed: "#e16d76"
    readonly property color terminalGreen: "#63c174"
    readonly property color terminalYellow: "#d8a657"
    readonly property color terminalBlue: "#6f8ff1"
    readonly property color terminalMagenta: "#c586c0"
    readonly property color terminalCyan: "#56b6c2"
    readonly property color terminalWhite: "#d4d4d4"
    readonly property color terminalBrightBlack: "#686868"
    readonly property color terminalBrightRed: "#f48771"
    readonly property color terminalBrightGreen: "#89d185"
    readonly property color terminalBrightYellow: "#e5c07b"
    readonly property color terminalBrightBlue: "#8aa6ff"
    readonly property color terminalBrightMagenta: "#d7a0d2"
    readonly property color terminalBrightCyan: "#7fdbdf"
    readonly property color terminalBrightWhite: "#f5f5f5"
    readonly property color terminalDimRed: "#8d464d"
    readonly property color terminalDimGreen: "#3f7a49"
    readonly property color terminalDimYellow: "#87693a"
    readonly property color terminalDimBlue: "#465b98"
    readonly property color terminalDimMagenta: "#795276"
    readonly property color terminalDimCyan: "#376f76"
    readonly property color terminalDimWhite: "#858585"
    readonly property color terminalDimForeground: muted

    function terminalColor(token) {
        switch (token) {
        case "terminalBackground": return terminalBackground
        case "terminalForeground": return terminalForeground
        case "terminalCursor": return terminalCursor
        case "terminalBlack": return terminalBlack
        case "terminalRed": return terminalRed
        case "terminalGreen": return terminalGreen
        case "terminalYellow": return terminalYellow
        case "terminalBlue": return terminalBlue
        case "terminalMagenta": return terminalMagenta
        case "terminalCyan": return terminalCyan
        case "terminalWhite": return terminalWhite
        case "terminalBrightBlack": return terminalBrightBlack
        case "terminalBrightRed": return terminalBrightRed
        case "terminalBrightGreen": return terminalBrightGreen
        case "terminalBrightYellow": return terminalBrightYellow
        case "terminalBrightBlue": return terminalBrightBlue
        case "terminalBrightMagenta": return terminalBrightMagenta
        case "terminalBrightCyan": return terminalBrightCyan
        case "terminalBrightWhite": return terminalBrightWhite
        case "terminalDimRed": return terminalDimRed
        case "terminalDimGreen": return terminalDimGreen
        case "terminalDimYellow": return terminalDimYellow
        case "terminalDimBlue": return terminalDimBlue
        case "terminalDimMagenta": return terminalDimMagenta
        case "terminalDimCyan": return terminalDimCyan
        case "terminalDimWhite": return terminalDimWhite
        case "terminalDimForeground": return terminalDimForeground
        default: return undefined
        }
    }

    function terminalMarkup(markup) {
        return markup.replace(/@([A-Za-z]+)@/g, function(match, token) {
            const color = terminalColor(token)
            return color === undefined ? match : String(color)
        })
    }

    readonly property int headerHeight: 48
    readonly property int sidebarWidth: 264
    readonly property int radius: 8
    readonly property int spacing: 12
    readonly property int compactRowHeight: 38
    readonly property int fileRowHeight: 50
    readonly property int aiThreadRowHeight: 58
    readonly property int diffRowHeight: 23
    readonly property int transitionDuration: 110
    readonly property string uiFont: Qt.platform.os === "osx" ? ".AppleSystemUIFont" : "Inter"
    readonly property string monoFont: Qt.platform.os === "osx" ? "Menlo" : "JetBrains Mono"
}
