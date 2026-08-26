pragma ComponentBehavior: Bound

import QtQuick
import QtTest
import Hunk

Item {
    id: root

    property string copiedText: ""
    width: 720
    height: 520

    Component {
        id: blockComponent

        AiMarkdownBlock {
            width: 640
            blockKind: "code"
            plainText: "fn main() {\n    println!(\"hi\");\n}"
            markup: "<font color=\"@keyword@\">fn</font><font color=\"@plain@\">&nbsp;main()&nbsp;{</font><br><font color=\"@plain@\">&nbsp;&nbsp;&nbsp;&nbsp;println!(</font><font color=\"@string@\">&quot;hi&quot;</font><font color=\"@plain@\">);</font><br><font color=\"@plain@\">}</font>"
            language: "rust"
            copyText: "fn main() {\n    println!(\"hi\");\n}"
            headingLevel: 0
            onCopyRequested: text => root.copiedText = text
        }
    }

    TestCase {
        name: "AiMarkdownBlock"
        when: windowShown

        function init() {
            root.copiedText = "";
        }

        function test_codeBlockRendersSyntaxAndCopiesSource() {
            const block = createTemporaryObject(blockComponent, root);
            verify(!!block, "Markdown block exists");
            const codeBlock = findChild(block, "aiMarkdownCodeBlock");
            const language = findChild(block, "aiMarkdownCodeLanguage");
            const codeText = findChild(block, "aiMarkdownCodeText");
            const copy = findChild(block, "aiMarkdownCopyButton");
            verify(!!codeBlock, "Code surface exists");
            verify(!!language, "Language label exists");
            verify(!!codeText, "Code text exists");
            verify(!!copy, "Copy action exists");
            compare(language.text, "RUST");
            verify(codeText.text.indexOf(String(Theme.syntaxKeyword)) >= 0);
            verify(codeText.getText(0, codeText.length).indexOf("fn main()") >= 0);

            mouseClick(copy);
            compare(root.copiedText, "fn main() {\n    println!(\"hi\");\n}");
        }
    }
}
