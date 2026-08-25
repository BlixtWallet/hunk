pragma ComponentBehavior: Bound

import QtQuick
import QtTest
import Hunk

Item {
    id: root
    width: 900
    height: 600
    property bool surfaceHasFrame: true

    ListModel {
        id: browserTabs
        ListElement {
            tab_id: "tab-1"
            title: "Hunk"
            url: "https://example.com"
            loading: false
        }
    }

    FakeBrowser {
        id: fakeBrowser
        tabs: browserTabs
        activeThreadId: "thread-browser"
        url: "https://example.com"
        title: "Hunk"
        canGoBack: true
        open: true
    }

    Component {
        id: fakeSurface
        Rectangle {
            property bool hasFrame: root.surfaceHasFrame
            objectName: "fakeBrowserSurface"
            color: Theme.input
        }
    }

    Component {
        id: browserPaneComponent
        BrowserPane {
            width: 900
            height: 600
            browser: fakeBrowser
            surfaceComponent: fakeSurface
        }
    }

    TestCase {
        name: "BrowserPaneTests"
        when: windowShown

        function init() {
            fakeBrowser.url = "https://example.com";
            fakeBrowser.activeTabId = "tab-1";
            fakeBrowser.activeTabIndex = 0;
            fakeBrowser.approvalPending = false;
            fakeBrowser.lastKeyPress = "";
            root.surfaceHasFrame = true;
            browserTabs.clear();
            browserTabs.append({
                tab_id: "tab-1",
                title: "Hunk",
                url: "https://example.com",
                loading: false
            });
        }

        function test_surfaceComponentIsInjected() {
            let pane = createTemporaryObject(browserPaneComponent, root);
            verify(!!pane, "Component exists");
            let loader = findChild(pane, "browserSurfaceLoader");
            verify(!!loader, "Object exists");
            tryCompare(loader, "status", Loader.Ready);
            compare(loader.item?.objectName, "fakeBrowserSurface");
        }

        function test_addressAcceptsCharactersNumbersAndSymbols() {
            let pane = createTemporaryObject(browserPaneComponent, root);
            verify(!!pane, "Component exists");
            let address = findChild(pane, "browserAddress");
            verify(!!address, "Object exists");
            address.focus = true;
            address.text = qsTr("docs.example.test/v2?q=qt&fps=120");
            compare(address.text, qsTr("docs.example.test/v2?q=qt&fps=120"));
        }

        function test_addressTracksBrowserStateAwayFromFocus() {
            let pane = createTemporaryObject(browserPaneComponent, root);
            verify(!!pane, "Component exists");
            let address = findChild(pane, "browserAddress");
            verify(!!address, "Object exists");
            address.focus = false;
            fakeBrowser.url = "https://qt.io/6.11";
            fakeBrowser.stateChanged();
            compare(address.text, qsTr("https://qt.io/6.11"));
        }

        function test_activeTabStaysInsideTheVisibleStrip() {
            let pane = createTemporaryObject(browserPaneComponent, root);
            verify(!!pane, "Component exists");
            for (let index = 2; index <= 10; ++index) {
                browserTabs.append({
                    tab_id: "tab-" + index,
                    title: "Tab " + index,
                    url: "https://example.com/" + index,
                    loading: false
                });
            }
            fakeBrowser.activeTabId = "tab-10";
            fakeBrowser.activeTabIndex = 9;
            fakeBrowser.stateChanged();
            tryCompare(pane.tabListView, "currentIndex", 9);
            tryVerify(() => pane.tabListView.contentX > 0);
            pane.tabListView.currentIndex = 0;
            browserTabs.append({
                tab_id: "tab-11",
                title: "Tab 11",
                url: "https://example.com/11",
                loading: false
            });
            tryCompare(pane.tabListView, "currentIndex", 9);
        }

        function test_browserFocusWaitsForApprovalResolution() {
            fakeBrowser.approvalPending = true;
            let pane = createTemporaryObject(browserPaneComponent, root);
            verify(!!pane, "Component exists");
            let input = findChild(pane, "browserInput");
            verify(!!input, "Object exists");
            pane.focusBrowserWhenReady();
            wait(0);
            verify(!input.activeFocus);
            fakeBrowser.approvalPending = false;
            fakeBrowser.approvalChanged();
            wait(0);
            verify(!input.activeFocus);
            pane.focusBrowserWhenReady();
            tryCompare(input, "activeFocus", true);
        }

        function test_inputWaitsForTheFirstNativeFrame() {
            root.surfaceHasFrame = false;
            let pane = createTemporaryObject(browserPaneComponent, root);
            verify(!!pane, "Component exists");
            let loader = findChild(pane, "browserSurfaceLoader");
            let input = findChild(pane, "browserInput");
            let pointerArea = findChild(pane, "browserPointerArea");
            tryCompare(loader, "status", Loader.Ready);
            verify(!pane.frameReady);
            verify(!input.enabled);
            verify(!pointerArea.enabled);
            root.surfaceHasFrame = true;
            tryCompare(pane, "frameReady", true);
            tryCompare(input, "enabled", true);
            tryCompare(pointerArea, "enabled", true);
        }

        function test_modifiedPrintableShortcutsReachCefKeyInput() {
            let pane = createTemporaryObject(browserPaneComponent, root);
            verify(!!pane, "Component exists");
            let input = findChild(pane, "browserInput");
            input.forceActiveFocus();
            tryCompare(input, "activeFocus", true);

            keyClick(Qt.Key_Space, Qt.ControlModifier);
            compare(fakeBrowser.lastKeyPress, "Control+Space");
            keyClick(Qt.Key_BracketLeft, Qt.MetaModifier);
            compare(fakeBrowser.lastKeyPress, "Meta+BracketLeft");
            keyClick(Qt.Key_Equal, Qt.ControlModifier | Qt.ShiftModifier);
            compare(fakeBrowser.lastKeyPress, "Control+Shift+Equal");
        }
    }
}
