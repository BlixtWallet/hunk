pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls

Item {
    id: root

    required property QtObject browser
    property Component surfaceComponent: null
    readonly property alias addressInput: addressInput
    readonly property alias surfaceLoader: surfaceLoader
    readonly property alias tabListView: tabs
    readonly property real deviceScale: Screen.devicePixelRatio
    readonly property bool frameReady: surfaceLoader.status === Loader.Ready && surfaceLoader.item !== null && surfaceLoader.item.hasFrame
    property bool clearingInput: false
    property string viewportTarget: ""
    property int pendingMouseX: 0
    property int pendingMouseY: 0
    property bool pendingMouseShift: false
    property bool pendingMouseControl: false
    property bool pendingMouseAlt: false
    property bool pendingMouseMeta: false

    onDeviceScaleChanged: scheduleResize()
    onFrameReadyChanged: {
        if (frameReady)
            focusBrowserWhenReady();
    }
    onVisibleChanged: {
        if (visible) {
            scheduleResize();
            focusBrowserWhenReady();
        } else {
            resizeTimer.stop();
            mouseMoveTimer.stop();
            contextMenu.close();
        }
    }
    Component.onCompleted: {
        scheduleResize();
        focusBrowserWhenReady();
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.canvas
    }

    Rectangle {
        id: tabsBar
        anchors {
            left: parent.left
            right: parent.right
            top: parent.top
        }
        height: 34
        color: Theme.chrome

        ListView {
            id: tabs
            anchors {
                left: parent.left
                right: newTabButton.left
                top: parent.top
                bottom: parent.bottom
            }
            orientation: ListView.Horizontal
            clip: true
            spacing: 1
            model: root.browser.tabs
            currentIndex: root.browser.activeTabIndex
            onCurrentIndexChanged: Qt.callLater(() => {
                if (currentIndex >= 0 && currentIndex < count)
                    positionViewAtIndex(currentIndex, ListView.Contain);
            })
            onCountChanged: restoreActiveIndex()

            function restoreActiveIndex() {
                currentIndex = Qt.binding(() => root.browser.activeTabIndex);
                Qt.callLater(() => {
                    if (currentIndex >= 0 && currentIndex < count)
                        positionViewAtIndex(currentIndex, ListView.Contain);
                });
            }

            delegate: Rectangle {
                id: tabDelegate
                required property string tab_id
                required property string title
                required property bool loading

                width: Math.min(210, Math.max(120, titleLabel.implicitWidth + 58))
                height: tabs.height
                color: root.browser.activeTabId === tab_id ? Theme.raised : Theme.chrome

                Text {
                    id: titleLabel
                    anchors {
                        left: parent.left
                        right: closeTab.left
                        leftMargin: 12
                        rightMargin: 5
                        verticalCenter: parent.verticalCenter
                    }
                    text: tabDelegate.loading ? "· " + tabDelegate.title : tabDelegate.title
                    textFormat: Text.PlainText
                    color: root.browser.activeTabId === tabDelegate.tab_id ? Theme.foreground : Theme.muted
                    elide: Text.ElideRight
                    font.family: Theme.uiFont
                    font.pixelSize: 11
                }

                ActionButton {
                    id: closeTab
                    anchors {
                        right: parent.right
                        rightMargin: 4
                        verticalCenter: parent.verticalCenter
                    }
                    label: "×"
                    compact: true
                    accessibleName: qsTr("Close browser tab")
                    onClicked: root.browser.close_tab(tabDelegate.tab_id)
                }

                MouseArea {
                    anchors.left: parent.left
                    anchors.right: closeTab.left
                    height: parent.height
                    onClicked: root.browser.select_tab(tabDelegate.tab_id)
                }

                Rectangle {
                    anchors.bottom: parent.bottom
                    width: parent.width
                    height: root.browser.activeTabId === tabDelegate.tab_id ? 2 : 1
                    color: root.browser.activeTabId === tabDelegate.tab_id ? Theme.accent : Theme.border
                }
            }
        }

        ActionButton {
            id: newTabButton
            anchors {
                right: parent.right
                rightMargin: 6
                verticalCenter: parent.verticalCenter
            }
            label: "+"
            compact: true
            accessibleName: qsTr("New browser tab")
            onClicked: root.browser.new_tab()
        }
    }

    Rectangle {
        id: toolbar
        anchors {
            left: parent.left
            right: parent.right
            top: tabsBar.bottom
        }
        height: 44
        color: Theme.raised

        Row {
            id: navigation
            anchors {
                left: parent.left
                leftMargin: 8
                verticalCenter: parent.verticalCenter
            }
            spacing: 4

            ActionButton {
                label: "←"
                compact: true
                enabled: root.browser.canGoBack
                onClicked: root.browser.go_back()
            }
            ActionButton {
                label: "→"
                compact: true
                enabled: root.browser.canGoForward
                onClicked: root.browser.go_forward()
            }
            ActionButton {
                label: root.browser.loading ? "×" : "↻"
                compact: true
                onClicked: root.browser.loading ? root.browser.stop() : root.browser.reload()
            }
        }

        Rectangle {
            anchors {
                left: navigation.right
                right: devtoolsButton.left
                leftMargin: 8
                rightMargin: 8
                verticalCenter: parent.verticalCenter
            }
            height: 30
            radius: Theme.radius
            color: Theme.input
            border.width: addressInput.activeFocus ? 1 : 0
            border.color: Theme.accent

            TextInput {
                id: addressInput
                objectName: "browserAddress"
                anchors.fill: parent
                anchors.leftMargin: 10
                anchors.rightMargin: 10
                verticalAlignment: TextInput.AlignVCenter
                color: Theme.foreground
                selectionColor: Theme.accentMuted
                selectedTextColor: Theme.foreground
                font.family: Theme.uiFont
                font.pixelSize: 11
                selectByMouse: true
                onAccepted: root.browser.navigate(text)
            }

            Binding {
                target: addressInput
                property: "text"
                value: root.browser.url
                when: !addressInput.activeFocus
                restoreMode: Binding.RestoreNone
            }
        }

        ActionButton {
            id: devtoolsButton
            anchors {
                right: parent.right
                rightMargin: 8
                verticalCenter: parent.verticalCenter
            }
            label: "DevTools"
            compact: true
            onClicked: root.browser.toggle_devtools()
        }

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.border
        }
    }

    Rectangle {
        id: statusBar
        anchors {
            left: parent.left
            right: parent.right
            top: toolbar.bottom
        }
        height: visible ? 25 : 0
        visible: root.browser.statusMessage.length > 0
        color: Theme.negativeMuted

        Text {
            anchors {
                left: parent.left
                right: parent.right
                leftMargin: 12
                rightMargin: 12
                verticalCenter: parent.verticalCenter
            }
            text: root.browser.statusMessage
            textFormat: Text.PlainText
            color: Theme.negative
            elide: Text.ElideRight
            font.family: Theme.uiFont
            font.pixelSize: 10
        }
    }

    Item {
        id: surface
        objectName: "browserSurface"
        anchors {
            left: parent.left
            right: parent.right
            top: statusBar.bottom
            bottom: parent.bottom
        }
        clip: true
        onWidthChanged: root.scheduleResize()
        onHeightChanged: root.scheduleResize()

        Loader {
            id: surfaceLoader
            objectName: "browserSurfaceLoader"
            anchors.fill: parent
            active: root.visible && root.browser.open && root.browser.runtimeStatus === "ready" && root.surfaceComponent !== null
            sourceComponent: root.surfaceComponent
            onLoaded: {
                root.browser.republish_frame();
                root.scheduleResize();
                root.focusBrowserWhenReady();
            }
        }

        Column {
            anchors.centerIn: parent
            width: Math.min(420, parent.width - 48)
            spacing: 8
            visible: !root.frameReady

            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: root.browser.runtimeStatus === "starting" ? qsTr("Starting Chromium…") : (surfaceLoader.status === Loader.Ready ? qsTr("Waiting for browser frame…") : qsTr("Browser surface unavailable"))
                color: Theme.foreground
                font {
                    family: Theme.uiFont
                    pixelSize: 15
                    weight: Font.DemiBold
                }
            }

            Text {
                width: parent.width
                text: root.browser.statusMessage
                textFormat: Text.PlainText
                color: Theme.muted
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.WordWrap
                font.family: Theme.uiFont
                font.pixelSize: 11
            }
        }

        TextInput {
            id: browserInput
            objectName: "browserInput"
            x: -2
            y: -2
            width: 1
            height: 1
            opacity: 0
            enabled: root.frameReady
            activeFocusOnTab: true
            Accessible.role: Accessible.EditableText
            Accessible.name: qsTr("Browser input")
            Keys.priority: Keys.BeforeItem

            onActiveFocusChanged: root.browser.report_focus(activeFocus)
            onTextChanged: {
                if (root.clearingInput || text.length === 0)
                    return;
                root.browser.text_input(text);
                root.clearingInput = true;
                text = "";
                root.clearingInput = false;
            }

            Keys.onPressed: event => {
                const key = root.browserKeyName(event);
                const shortcutModifier = root.modifierActive(event.modifiers, Qt.MetaModifier) || root.modifierActive(event.modifiers, Qt.ControlModifier) || root.modifierActive(event.modifiers, Qt.AltModifier);
                const special = key.length > 0 && (shortcutModifier || !root.browserKeyIsPrintable(key));
                if (special)
                    event.accepted = root.browser.key_press(root.browserKeySequence(event, key));
            }
        }

        MouseArea {
            objectName: "browserPointerArea"
            anchors.fill: parent
            enabled: root.frameReady
            acceptedButtons: Qt.AllButtons
            hoverEnabled: true
            cursorShape: Qt.ArrowCursor

            onPressed: mouse => {
                root.focusBrowser();
                const button = mouse.button === Qt.RightButton ? "right" : (mouse.button === Qt.MiddleButton ? "middle" : "left");
                mouse.accepted = root.browser.mouse_click(Math.round(mouse.x), Math.round(mouse.y), button, root.modifierActive(mouse.modifiers, Qt.ShiftModifier), root.modifierActive(mouse.modifiers, Qt.ControlModifier), root.modifierActive(mouse.modifiers, Qt.AltModifier), root.modifierActive(mouse.modifiers, Qt.MetaModifier));
            }
            onPositionChanged: mouse => root.queueMouseMove(mouse)
            onWheel: wheel => {
                const deltaX = wheel.pixelDelta.x !== 0 ? wheel.pixelDelta.x : wheel.angleDelta.x / 2;
                const deltaY = wheel.pixelDelta.y !== 0 ? wheel.pixelDelta.y : wheel.angleDelta.y / 2;
                wheel.accepted = root.browser.wheel(Math.round(wheel.x), Math.round(wheel.y), Math.round(deltaX), Math.round(deltaY), root.modifierActive(wheel.modifiers, Qt.ShiftModifier), root.modifierActive(wheel.modifiers, Qt.ControlModifier), root.modifierActive(wheel.modifiers, Qt.AltModifier), root.modifierActive(wheel.modifiers, Qt.MetaModifier));
            }
        }
    }

    TextEdit {
        id: clipboardProxy
        x: -2
        y: -2
        width: 1
        height: 1
        opacity: 0
        readOnly: true
        textFormat: TextEdit.PlainText
    }

    Menu {
        id: contextMenu

        MenuItem {
            text: qsTr("Back")
            enabled: root.browser.canGoBack
            onTriggered: root.browser.context_action("back")
        }
        MenuItem {
            text: qsTr("Forward")
            enabled: root.browser.canGoForward
            onTriggered: root.browser.context_action("forward")
        }
        MenuItem {
            text: qsTr("Reload")
            onTriggered: root.browser.context_action("reload")
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("Open link in new tab")
            onTriggered: root.browser.context_action("open-link")
        }
        MenuItem {
            text: qsTr("Open media in new tab")
            onTriggered: root.browser.context_action("open-media")
        }
        MenuItem {
            text: qsTr("Copy page address")
            onTriggered: root.browser.context_action("copy-page")
        }
        MenuItem {
            text: qsTr("Copy link address")
            onTriggered: root.browser.context_action("copy-link")
        }
        MenuItem {
            text: qsTr("Copy media address")
            onTriggered: root.browser.context_action("copy-media")
        }
        MenuItem {
            text: qsTr("Copy selection")
            onTriggered: root.browser.context_action("copy-selection")
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("Cut")
            onTriggered: root.browser.context_action("cut")
        }
        MenuItem {
            text: qsTr("Copy")
            onTriggered: root.browser.context_action("copy")
        }
        MenuItem {
            text: qsTr("Paste")
            onTriggered: root.browser.context_action("paste")
        }
        MenuItem {
            text: qsTr("Select all")
            onTriggered: root.browser.context_action("select-all")
        }
        MenuSeparator {}
        MenuItem {
            text: qsTr("Inspect")
            onTriggered: root.browser.context_action("inspect")
        }
    }

    Timer {
        id: resizeTimer
        interval: 40
        repeat: false
        onTriggered: {
            if (root.visible && root.browser.open && root.browser.runtimeStatus === "ready" && surfaceLoader.status === Loader.Ready && surface.width > 0 && surface.height > 0) {
                root.browser.resize(Math.round(surface.width), Math.round(surface.height), root.deviceScale);
            }
        }
    }

    Timer {
        id: mouseMoveTimer
        interval: 8
        repeat: false
        onTriggered: root.browser.mouse_move(root.pendingMouseX, root.pendingMouseY, root.pendingMouseShift, root.pendingMouseControl, root.pendingMouseAlt, root.pendingMouseMeta)
    }

    Connections {
        target: root.browser

        function onStateChanged() {
            const target = root.browser.activeThreadId + "\n" + root.browser.activeTabId;
            if (target !== root.viewportTarget) {
                root.viewportTarget = target;
                contextMenu.close();
                root.scheduleResize();
            }
        }

        function onContextMenuRequested(x, y) {
            contextMenu.x = Math.max(0, Math.min(x, root.width - contextMenu.width));
            contextMenu.y = Math.max(0, Math.min(statusBar.y + statusBar.height + y, root.height - contextMenu.height));
            contextMenu.open();
        }

        function onContextChanged() {
            const value = root.browser.contextClipboardText;
            if (value.length === 0 || clipboardProxy.text === value)
                return;
            clipboardProxy.text = value;
            clipboardProxy.forceActiveFocus();
            clipboardProxy.selectAll();
            clipboardProxy.copy();
            clipboardProxy.deselect();
            root.focusBrowser();
        }
    }

    function modifierActive(modifiers, modifier) {
        return (modifiers & modifier) !== 0;
    }

    function browserKeyName(event) {
        switch (event.key) {
        case Qt.Key_Return:
        case Qt.Key_Enter:
            return "Enter";
        case Qt.Key_Tab:
        case Qt.Key_Backtab:
            return "Tab";
        case Qt.Key_Backspace:
            return "Backspace";
        case Qt.Key_Escape:
            return "Escape";
        case Qt.Key_Up:
            return "ArrowUp";
        case Qt.Key_Down:
            return "ArrowDown";
        case Qt.Key_Left:
            return "ArrowLeft";
        case Qt.Key_Right:
            return "ArrowRight";
        case Qt.Key_Home:
            return "Home";
        case Qt.Key_End:
            return "End";
        case Qt.Key_PageUp:
            return "PageUp";
        case Qt.Key_PageDown:
            return "PageDown";
        case Qt.Key_Delete:
            return "Delete";
        case Qt.Key_Insert:
            return "Insert";
        case Qt.Key_Space:
            return "Space";
        case Qt.Key_Equal:
        case Qt.Key_Plus:
            return "Equal";
        case Qt.Key_Minus:
        case Qt.Key_Underscore:
            return "Minus";
        case Qt.Key_BracketLeft:
        case Qt.Key_BraceLeft:
            return "BracketLeft";
        case Qt.Key_BracketRight:
        case Qt.Key_BraceRight:
            return "BracketRight";
        case Qt.Key_Slash:
        case Qt.Key_Question:
            return "Slash";
        case Qt.Key_Comma:
        case Qt.Key_Less:
            return "Comma";
        case Qt.Key_Period:
        case Qt.Key_Greater:
            return "Period";
        case Qt.Key_Semicolon:
        case Qt.Key_Colon:
            return "Semicolon";
        case Qt.Key_Apostrophe:
        case Qt.Key_QuoteDbl:
            return "Apostrophe";
        case Qt.Key_Backslash:
        case Qt.Key_Bar:
            return "Backslash";
        case Qt.Key_QuoteLeft:
        case Qt.Key_AsciiTilde:
            return "Backtick";
        case Qt.Key_Exclam:
            return "1";
        case Qt.Key_At:
            return "2";
        case Qt.Key_NumberSign:
            return "3";
        case Qt.Key_Dollar:
            return "4";
        case Qt.Key_Percent:
            return "5";
        case Qt.Key_AsciiCircum:
            return "6";
        case Qt.Key_Ampersand:
            return "7";
        case Qt.Key_Asterisk:
            return "8";
        case Qt.Key_ParenLeft:
            return "9";
        case Qt.Key_ParenRight:
            return "0";
        }
        if (event.key >= Qt.Key_F1 && event.key <= Qt.Key_F12)
            return "F" + (event.key - Qt.Key_F1 + 1);
        if (event.key >= Qt.Key_A && event.key <= Qt.Key_Z)
            return String.fromCharCode(event.key);
        if (event.key >= Qt.Key_0 && event.key <= Qt.Key_9)
            return String.fromCharCode(event.key);
        return "";
    }

    function browserKeyIsPrintable(key) {
        return key.length === 1 || key === "Space" || key === "Equal" || key === "Minus" || key === "BracketLeft" || key === "BracketRight" || key === "Slash" || key === "Comma" || key === "Period" || key === "Semicolon" || key === "Apostrophe" || key === "Backslash" || key === "Backtick";
    }

    function browserKeySequence(event, key) {
        if (key.length === 0)
            return "";
        const parts = [];
        if (modifierActive(event.modifiers, Qt.MetaModifier))
            parts.push("Meta");
        if (modifierActive(event.modifiers, Qt.ControlModifier))
            parts.push("Control");
        if (modifierActive(event.modifiers, Qt.AltModifier))
            parts.push("Alt");
        if (modifierActive(event.modifiers, Qt.ShiftModifier))
            parts.push("Shift");
        parts.push(key);
        return parts.join("+");
    }

    function focusBrowserWhenReady() {
        Qt.callLater(() => {
            if (root.visible && root.browser.open && root.frameReady && !root.browser.approvalPending)
                browserInput.forceActiveFocus();
        });
    }

    function queueMouseMove(mouse) {
        root.pendingMouseX = Math.round(mouse.x);
        root.pendingMouseY = Math.round(mouse.y);
        root.pendingMouseShift = root.modifierActive(mouse.modifiers, Qt.ShiftModifier);
        root.pendingMouseControl = root.modifierActive(mouse.modifiers, Qt.ControlModifier);
        root.pendingMouseAlt = root.modifierActive(mouse.modifiers, Qt.AltModifier);
        root.pendingMouseMeta = root.modifierActive(mouse.modifiers, Qt.MetaModifier);
        if (!mouseMoveTimer.running)
            mouseMoveTimer.start();
    }

    function focusBrowser() {
        if (root.visible && root.browser.open && root.frameReady && !root.browser.approvalPending)
            browserInput.forceActiveFocus();
    }

    function scheduleResize() {
        if (root.visible && root.browser.open && root.browser.runtimeStatus === "ready" && surfaceLoader.status === Loader.Ready)
            resizeTimer.restart();
    }
}
