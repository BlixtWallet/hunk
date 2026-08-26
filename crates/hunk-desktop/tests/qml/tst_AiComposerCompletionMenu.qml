pragma ComponentBehavior: Bound

import QtQuick
import QtTest
import Hunk

Item {
    id: root

    width: 640
    height: 480

    Component {
        id: menuComponent

        AiComposerCompletionMenu {
            width: 500
            height: implicitHeight
            items: [
                {
                    kind: "command",
                    value: "code",
                    label: "/code",
                    description: "Switch to standard coding mode.",
                    disabled: false
                },
                {
                    kind: "file",
                    value: "src/main.rs",
                    label: "main.rs",
                    description: "src/main.rs",
                    disabled: false
                }
            ]
        }
    }

    Component {
        id: signalSpy
        SignalSpy {}
    }

    TestCase {
        name: "AiComposerCompletionMenu"
        when: windowShown

        function test_sizesFromVisibleRows() {
            const menu = createTemporaryObject(menuComponent, root)
            verify(!!menu, "Component exists")
            compare(menu.height, 88)
            compare(menu.visible, true)
        }

        function test_clickAcceptsCompletion() {
            const menu = createTemporaryObject(menuComponent, root)
            verify(!!menu, "Component exists")
            const spy = signalSpy.createObject(root, {
                target: menu,
                signalName: "accepted"
            })
            verify(!!spy, "Signal spy exists")

            mouseClick(menu, 20, 20)

            tryCompare(spy, "count", 1)
            compare(spy.signalArguments[0][0], "command")
            compare(spy.signalArguments[0][1], "code")
        }
    }
}
