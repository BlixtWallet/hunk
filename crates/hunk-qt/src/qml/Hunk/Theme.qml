pragma Singleton

import QtQuick

QtObject {
    readonly property color canvas: "#1e1e1e"
    readonly property color chrome: "#252526"
    readonly property color raised: "#2d2d30"
    readonly property color hover: "#37373d"
    readonly property color border: "#3e3e42"
    readonly property color accent: "#5f81eb"
    readonly property color accentStrong: "#7d9eff"
    readonly property color foreground: "#eeeeec"
    readonly property color muted: "#969696"
    readonly property color faint: "#666666"
    readonly property color positive: "#63c174"
    readonly property color negative: "#e16d76"

    readonly property int headerHeight: 48
    readonly property int sidebarWidth: 252
    readonly property int radius: 8
    readonly property int spacing: 12
    readonly property int transitionDuration: 110
    readonly property string uiFont: Qt.platform.os === "osx" ? ".AppleSystemUIFont" : "Inter"
    readonly property string monoFont: Qt.platform.os === "osx" ? "Menlo" : "JetBrains Mono"
}
