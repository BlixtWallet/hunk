pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property var backend
    focus: true
    readonly property alias diffListView: diffList
    readonly property alias searchInput: searchInput
    readonly property alias commentsInspector: commentsInspector
    readonly property var commentComposer: commentComposerLoader.item
    readonly property bool loadingStateVisible: loadingState.visible
    readonly property bool errorStateVisible: errorState.visible
    readonly property bool emptyStateVisible: emptyState.visible
    readonly property int selectionStart: selectionAnchorRow < 0 || selectionHeadRow < 0
        ? -1 : Math.min(selectionAnchorRow, selectionHeadRow)
    readonly property int selectionEnd: selectionAnchorRow < 0 || selectionHeadRow < 0
        ? -1 : Math.max(selectionAnchorRow, selectionHeadRow)
    property bool unifiedMode: false
    property bool commentsInspectorOpen: false
    property int activeCommentRow: -1
    property int lastCommentTargetRevision: 0
    property int selectionAnchorRow: -1
    property int selectionHeadRow: -1
    readonly property string selectionPath: backend.diffSelectedPath

    onSelectionPathChanged: {
        resetSelection()
        closeCommentComposer()
    }

    function resetSelection() {
        selectionAnchorRow = -1
        selectionHeadRow = -1
        diffList.currentIndex = -1
    }

    function clampSelection() {
        if (diffList.count <= 0) {
            resetSelection()
            return
        }
        if (selectionAnchorRow >= diffList.count)
            selectionAnchorRow = diffList.count - 1
        if (selectionHeadRow >= diffList.count)
            selectionHeadRow = diffList.count - 1
    }

    function rowIsSelected(index) {
        return selectionStart >= 0 && index >= selectionStart && index <= selectionEnd
    }

    function selectRow(index, extendSelection) {
        if (diffList.count <= 0)
            return
        const target = Math.max(0, Math.min(index, diffList.count - 1))
        if (extendSelection && selectionAnchorRow >= 0)
            selectionHeadRow = target
        else {
            selectionAnchorRow = target
            selectionHeadRow = target
        }
        diffList.currentIndex = target
        diffList.positionViewAtIndex(target, ListView.Contain)
        forceActiveFocus()
    }

    function moveSelection(delta, extendSelection) {
        const base = selectionHeadRow >= 0
            ? selectionHeadRow : (delta > 0 ? -1 : 0)
        selectRow(base + delta, extendSelection)
    }

    function selectAllRows() {
        if (diffList.count <= 0)
            return
        selectionAnchorRow = 0
        selectionHeadRow = diffList.count - 1
        diffList.currentIndex = selectionHeadRow
        forceActiveFocus()
    }

    function jumpHunk(direction) {
        const target = backend.diff_hunk_target(selectionHeadRow, direction)
        if (target >= 0)
            selectRow(target, false)
    }

    function copySelection() {
        const selectedText = backend.diff_selection_text(selectionAnchorRow, selectionHeadRow)
        copyText(selectedText)
    }

    function copyText(text) {
        if (text.length === 0)
            return
        clipboardProxy.text = text
        clipboardProxy.forceActiveFocus()
        clipboardProxy.selectAll()
        clipboardProxy.copy()
        clipboardProxy.deselect()
        forceActiveFocus()
    }

    function openCommentComposer(row) {
        if (row < 0 || !backend.diff_row_supports_comments(row))
            return
        selectRow(row, false)
        activeCommentRow = row
    }

    function closeCommentComposer() {
        activeCommentRow = -1
        forceActiveFocus()
    }

    function toggleCommentsInspector() {
        commentsInspectorOpen = !commentsInspectorOpen
        if (commentsInspectorOpen)
            backend.refresh_diff_comments()
    }

    function commentComposerY(scrollPosition) {
        const viewportTop = columnHeader.y + columnHeader.height + 8
        const viewportBottom = height - commentComposerLoader.height - 8
        const rowItem = diffList.itemAtIndex(activeCommentRow)
        if (rowItem === null)
            return viewportTop
        const rowBottom = horizontalViewport.y + rowItem.y
            - scrollPosition + rowItem.height
        return Math.max(viewportTop, Math.min(rowBottom + 5, viewportBottom))
    }

    function applyCommentTarget() {
        if (backend.diffCommentTargetRevision === lastCommentTargetRevision)
            return
        lastCommentTargetRevision = backend.diffCommentTargetRevision
        if (backend.diffCommentTargetRow >= 0)
            selectRow(backend.diffCommentTargetRow, false)
    }

    function setDiffMode(mode) {
        unifiedMode = mode === "unified"
    }

    function positionSearchTarget() {
        if (backend.diffSearchTargetRow < 0)
            return
        diffList.currentIndex = backend.diffSearchTargetRow
        diffList.positionViewAtIndex(backend.diffSearchTargetRow, ListView.Center)
    }

    function applySearch(query) {
        backend.set_diff_search(query)
        positionSearchTarget()
    }

    function moveSearch(direction) {
        backend.move_diff_search_match(direction)
        positionSearchTarget()
    }

    function escapeCode(text) {
        return String(text)
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/\"/g, "&quot;")
            .replace(/'/g, "&#39;")
            .replace(/@/g, "&#64;")
            .replace(/ /g, "&nbsp;")
            .replace(/\t/g, "&nbsp;&nbsp;&nbsp;&nbsp;")
    }

    function renderMarkup(markup, fallback) {
        let rendered = markup.length > 0
            ? markup : "<font color=\"@plain@\">" + escapeCode(fallback) + "</font>"
        const colors = {
            plain: Theme.foreground,
            keyword: Theme.syntaxKeyword,
            string: Theme.syntaxString,
            number: Theme.syntaxNumber,
            comment: Theme.syntaxComment,
            function: Theme.syntaxFunction,
            type: Theme.syntaxType,
            constant: Theme.syntaxConstant,
            variable: Theme.syntaxVariable,
            operator: Theme.syntaxOperator
        }
        for (const token in colors)
            rendered = rendered.split("@" + token + "@").join(String(colors[token]))
        return rendered
    }

    function cellColor(kind) {
        if (kind === "added")
            return Theme.positiveMuted
        if (kind === "removed")
            return Theme.negativeMuted
        return Theme.canvas
    }

    function markerColor(kind) {
        if (kind === "added")
            return Theme.positive
        if (kind === "removed")
            return Theme.negative
        return Theme.faint
    }

    Keys.priority: Keys.BeforeItem
    Keys.onPressed: event => {
        if (searchInput.activeFocus
                || (commentComposer !== null && commentComposer.editor.activeFocus))
            return
        const extendSelection = (event.modifiers & Qt.ShiftModifier) !== 0
        const commandModifier = (event.modifiers
            & (Qt.ControlModifier | Qt.MetaModifier)) !== 0
        if (event.key === Qt.Key_Down) {
            moveSelection(1, extendSelection)
            event.accepted = true
        } else if (event.key === Qt.Key_Up) {
            moveSelection(-1, extendSelection)
            event.accepted = true
        } else if (event.key === Qt.Key_F7) {
            jumpHunk(extendSelection ? -1 : 1)
            event.accepted = true
        } else if (commandModifier && event.key === Qt.Key_A) {
            selectAllRows()
            event.accepted = true
        } else if (commandModifier && event.key === Qt.Key_C) {
            copySelection()
            event.accepted = true
        }
    }

    component UnifiedLine: Rectangle {
        required property string cellKind
        required property int lineNumber
        required property string codeText
        required property string codeMarkup

        color: root.cellColor(cellKind)

        Text {
            anchors.left: parent.left
            anchors.leftMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            width: 38
            text: parent.lineNumber > 0 ? parent.lineNumber : ""
            color: Theme.faint
            horizontalAlignment: Text.AlignRight
            font.family: Theme.monoFont
            font.pixelSize: 9
        }

        Text {
            anchors.left: parent.left
            anchors.leftMargin: 52
            anchors.verticalCenter: parent.verticalCenter
            text: parent.cellKind === "removed" ? "−"
                : (parent.cellKind === "added" ? "+" : " ")
            color: root.markerColor(parent.cellKind)
            font.family: Theme.monoFont
            font.pixelSize: 11
        }

        Text {
            anchors.left: parent.left
            anchors.leftMargin: 68
            anchors.verticalCenter: parent.verticalCenter
            text: root.renderMarkup(parent.codeMarkup, parent.codeText)
            color: Theme.foreground
            textFormat: Text.StyledText
            font.family: Theme.monoFont
            font.pixelSize: 11
        }
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.canvas
    }

    Item {
        id: toolbar
        anchors.left: parent.left
        anchors.right: commentsInspector.left
        anchors.top: parent.top
        height: 52

        Column {
            anchors.left: parent.left
            anchors.leftMargin: 18
            anchors.right: actions.left
            anchors.rightMargin: 18
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2

            Text {
                width: parent.width
                text: root.backend.diffSelectedPath.length > 0
                    ? root.backend.diffSelectedPath : "Working tree"
                color: Theme.foreground
                elide: Text.ElideMiddle
                font.family: Theme.monoFont
                font.pixelSize: 12
                font.weight: Font.DemiBold
            }

            Row {
                spacing: 8

                Text {
                    text: root.backend.diffStatusTag.length > 0
                        ? root.backend.diffStatusTag : "—"
                    color: Theme.accentStrong
                    font.family: Theme.monoFont
                    font.pixelSize: 9
                    font.weight: Font.Bold
                }

                Text {
                    text: "+" + root.backend.diffAdditions
                    color: Theme.positive
                    font.family: Theme.monoFont
                    font.pixelSize: 9
                }

                Text {
                    text: "−" + root.backend.diffRemovals
                    color: Theme.negative
                    font.family: Theme.monoFont
                    font.pixelSize: 9
                }
            }
        }

        Row {
            id: actions
            anchors.right: parent.right
            anchors.rightMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            spacing: 7

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 186
                height: 26
                radius: 5
                color: Theme.input
                border.width: searchInput.activeFocus ? 1 : 0
                border.color: Theme.accentStrong

                TextInput {
                    id: searchInput
                    objectName: "diffSearchInput"
                    anchors.left: parent.left
                    anchors.right: searchCount.left
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    anchors.leftMargin: 8
                    anchors.rightMargin: 6
                    text: root.backend.diffSearchQuery
                    color: Theme.foreground
                    selectionColor: Theme.accent
                    selectedTextColor: Theme.foreground
                    clip: true
                    font.family: Theme.uiFont
                    font.pixelSize: 10
                    verticalAlignment: TextInput.AlignVCenter
                    onTextEdited: root.applySearch(text)

                    Keys.onPressed: event => {
                        if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                            root.moveSearch(event.modifiers & Qt.ShiftModifier ? -1 : 1)
                            event.accepted = true
                        } else if (event.key === Qt.Key_Escape) {
                            root.applySearch("")
                            event.accepted = true
                        }
                    }
                }

                Text {
                    anchors.left: searchInput.left
                    anchors.verticalCenter: parent.verticalCenter
                    visible: searchInput.text.length === 0 && !searchInput.activeFocus
                    text: "Search diff"
                    color: Theme.faint
                    font.family: Theme.uiFont
                    font.pixelSize: 10
                }

                Text {
                    id: searchCount
                    anchors.right: parent.right
                    anchors.rightMargin: 7
                    anchors.verticalCenter: parent.verticalCenter
                    text: root.backend.diffSearchMatchCount > 0
                        ? (root.backend.diffSearchMatchIndex + 1) + "/"
                            + root.backend.diffSearchMatchCount
                        : (searchInput.text.length > 0 ? "0/0" : "")
                    color: Theme.faint
                    font.family: Theme.monoFont
                    font.pixelSize: 8
                }
            }

            ActionButton {
                label: "↑"
                compact: true
                enabled: root.backend.diffSearchMatchCount > 0
                onClicked: root.moveSearch(-1)
            }

            ActionButton {
                label: "↓"
                compact: true
                enabled: root.backend.diffSearchMatchCount > 0
                onClicked: root.moveSearch(1)
            }

            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: root.backend.diffLoading
                text: "LOADING"
                color: Theme.faint
                font.family: Theme.monoFont
                font.pixelSize: 9
                font.letterSpacing: 0.7
            }

            ActionButton {
                label: "Split"
                compact: true
                primary: !root.unifiedMode
                onClicked: root.setDiffMode("split")
            }

            ActionButton {
                label: "Unified"
                compact: true
                primary: root.unifiedMode
                onClicked: root.setDiffMode("unified")
            }

            ActionButton {
                label: "Comment"
                compact: true
                enabled: root.selectionHeadRow >= 0
                    && root.backend.diff_row_supports_comments(root.selectionHeadRow)
                    && !root.backend.diffCommentsLoading
                    && !root.backend.diffCommentsBusy
                onClicked: root.openCommentComposer(root.selectionHeadRow)
            }

            ActionButton {
                label: root.backend.diffCommentsOpenCount > 0
                    ? "Comments " + root.backend.diffCommentsOpenCount : "Comments"
                compact: true
                primary: root.commentsInspectorOpen
                onClicked: root.toggleCommentsInspector()
            }

            ActionButton {
                label: "Refresh"
                compact: true
                enabled: root.backend.diffSelectedPath.length > 0 && !root.backend.diffLoading
                onClicked: root.backend.refresh_diff()
            }
        }

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.border
        }
    }

    Item {
        id: columnHeader
        anchors.left: parent.left
        anchors.right: commentsInspector.left
        anchors.top: toolbar.bottom
        height: 28
        visible: root.backend.diffSelectedPath.length > 0

        Rectangle {
            anchors.fill: parent
            color: Theme.chrome
        }

        Text {
            anchors.left: parent.left
            anchors.leftMargin: 58
            anchors.verticalCenter: parent.verticalCenter
            text: "BEFORE"
            visible: !root.unifiedMode
            color: Theme.muted
            font.family: Theme.monoFont
            font.pixelSize: 9
            font.weight: Font.DemiBold
            font.letterSpacing: 0.8
        }

        Text {
            anchors.left: parent.horizontalCenter
            anchors.leftMargin: 58
            anchors.verticalCenter: parent.verticalCenter
            text: "AFTER"
            visible: !root.unifiedMode
            color: Theme.muted
            font.family: Theme.monoFont
            font.pixelSize: 9
            font.weight: Font.DemiBold
            font.letterSpacing: 0.8
        }

        Rectangle {
            anchors.left: parent.horizontalCenter
            width: 1
            height: parent.height
            color: Theme.borderStrong
            visible: !root.unifiedMode
        }

        Text {
            anchors.left: parent.left
            anchors.leftMargin: 58
            anchors.verticalCenter: parent.verticalCenter
            text: "UNIFIED"
            visible: root.unifiedMode
            color: Theme.muted
            font.family: Theme.monoFont
            font.pixelSize: 9
            font.weight: Font.DemiBold
            font.letterSpacing: 0.8
        }

        Rectangle {
            anchors.bottom: parent.bottom
            width: parent.width
            height: 1
            color: Theme.border
        }
    }

    Flickable {
        id: horizontalViewport
        anchors.left: parent.left
        anchors.right: commentsInspector.left
        anchors.top: columnHeader.bottom
        anchors.bottom: parent.bottom
        contentWidth: Math.max(width, 1440)
        contentHeight: height
        flickableDirection: Flickable.HorizontalFlick
        boundsBehavior: Flickable.StopAtBounds
        clip: true

        ListView {
            id: diffList
            objectName: "diffRowList"
            width: horizontalViewport.contentWidth
            height: horizontalViewport.height
            model: root.backend.diffRows
            clip: true
            reuseItems: true
            cacheBuffer: Theme.diffRowHeight * 12
            boundsBehavior: Flickable.StopAtBounds
            onCountChanged: root.clampSelection()

            delegate: Item {
                id: diffRow

                required property int index
                required property string stable_id
                required property string row_kind
                required property int left_line
                required property string left_text
                required property string left_markup
                required property string left_kind
                required property int right_line
                required property string right_text
                required property string right_markup
                required property string right_kind
                required property string text

                readonly property bool pairedChange: left_kind === "removed"
                    && right_kind === "added"
                readonly property string unifiedPrimaryKind: left_kind === "removed"
                    ? left_kind : (right_kind !== "none" ? right_kind : left_kind)
                readonly property int unifiedPrimaryLine: left_kind === "removed"
                    ? left_line : (right_kind !== "none" ? right_line : left_line)
                readonly property string unifiedPrimaryText: left_kind === "removed"
                    ? left_text : (right_kind !== "none" ? right_text : left_text)
                readonly property string unifiedPrimaryMarkup: left_kind === "removed"
                    ? left_markup : (right_kind !== "none" ? right_markup : left_markup)
                readonly property int commentCount: {
                    const version = root.backend.diffCommentsVersion
                    return version >= 0
                        ? root.backend.diff_comment_count_for_row(diffRow.index) : 0
                }

                width: diffList.width
                height: diffRow.row_kind === "code"
                    ? (root.unifiedMode && diffRow.pairedChange
                        ? Theme.diffRowHeight * 2 : Theme.diffRowHeight)
                    : (diffRow.row_kind === "hunk" ? 32 : 54)

                Rectangle {
                    anchors.fill: parent
                    visible: diffRow.row_kind !== "code"
                    color: diffRow.row_kind === "hunk" ? Theme.raised : Theme.canvas

                    Text {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.leftMargin: 18
                        anchors.rightMargin: 18
                        anchors.verticalCenter: parent.verticalCenter
                        text: diffRow.text
                        color: diffRow.row_kind === "hunk" ? Theme.accentStrong : Theme.muted
                        elide: Text.ElideRight
                        font.family: Theme.monoFont
                        font.pixelSize: diffRow.row_kind === "hunk" ? 10 : 11
                    }

                    Rectangle {
                        anchors.bottom: parent.bottom
                        width: parent.width
                        height: 1
                        color: Theme.border
                    }
                }

                Item {
                    anchors.fill: parent
                    visible: diffRow.row_kind === "code" && !root.unifiedMode

                    Rectangle {
                        anchors.left: parent.left
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        width: parent.width / 2
                        color: root.cellColor(diffRow.left_kind)
                    }

                    Rectangle {
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        width: parent.width / 2
                        color: root.cellColor(diffRow.right_kind)
                    }

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 8
                        anchors.verticalCenter: parent.verticalCenter
                        width: 38
                        text: diffRow.left_line > 0 ? diffRow.left_line : ""
                        color: Theme.faint
                        horizontalAlignment: Text.AlignRight
                        font.family: Theme.monoFont
                        font.pixelSize: 9
                    }

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 52
                        anchors.verticalCenter: parent.verticalCenter
                        text: diffRow.left_kind === "removed" ? "−" : " "
                        color: root.markerColor(diffRow.left_kind)
                        font.family: Theme.monoFont
                        font.pixelSize: 11
                    }

                    Text {
                        anchors.left: parent.left
                        anchors.leftMargin: 68
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.renderMarkup(diffRow.left_markup, diffRow.left_text)
                        color: Theme.foreground
                        textFormat: Text.StyledText
                        font.family: Theme.monoFont
                        font.pixelSize: 11
                    }

                    Text {
                        anchors.left: parent.horizontalCenter
                        anchors.leftMargin: 8
                        anchors.verticalCenter: parent.verticalCenter
                        width: 38
                        text: diffRow.right_line > 0 ? diffRow.right_line : ""
                        color: Theme.faint
                        horizontalAlignment: Text.AlignRight
                        font.family: Theme.monoFont
                        font.pixelSize: 9
                    }

                    Text {
                        anchors.left: parent.horizontalCenter
                        anchors.leftMargin: 52
                        anchors.verticalCenter: parent.verticalCenter
                        text: diffRow.right_kind === "added" ? "+" : " "
                        color: root.markerColor(diffRow.right_kind)
                        font.family: Theme.monoFont
                        font.pixelSize: 11
                    }

                    Text {
                        anchors.left: parent.horizontalCenter
                        anchors.leftMargin: 68
                        anchors.verticalCenter: parent.verticalCenter
                        text: root.renderMarkup(diffRow.right_markup, diffRow.right_text)
                        color: Theme.foreground
                        textFormat: Text.StyledText
                        font.family: Theme.monoFont
                        font.pixelSize: 11
                    }

                    Rectangle {
                        anchors.left: parent.horizontalCenter
                        width: 1
                        height: parent.height
                        color: Theme.borderStrong
                    }
                }

                Item {
                    anchors.fill: parent
                    visible: diffRow.row_kind === "code" && root.unifiedMode

                    UnifiedLine {
                        id: unifiedPrimary
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        height: Theme.diffRowHeight
                        cellKind: diffRow.unifiedPrimaryKind
                        lineNumber: diffRow.unifiedPrimaryLine
                        codeText: diffRow.unifiedPrimaryText
                        codeMarkup: diffRow.unifiedPrimaryMarkup
                    }

                    UnifiedLine {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: unifiedPrimary.bottom
                        height: diffRow.pairedChange ? Theme.diffRowHeight : 0
                        visible: diffRow.pairedChange
                        cellKind: diffRow.right_kind
                        lineNumber: diffRow.right_line
                        codeText: diffRow.right_text
                        codeMarkup: diffRow.right_markup
                    }
                }

                Rectangle {
                    anchors.fill: parent
                    z: 5
                    visible: root.backend.diffSearchTargetRow === diffRow.index
                    color: Theme.transparent
                    border.width: 1
                    border.color: Theme.warning
                }

                Rectangle {
                    anchors.fill: parent
                    z: 4
                    visible: root.rowIsSelected(diffRow.index)
                    color: Qt.rgba(
                        Theme.selected.r,
                        Theme.selected.g,
                        Theme.selected.b,
                        0.28
                    )
                    border.width: 2
                    border.color: Theme.accentStrong
                }

                Rectangle {
                    objectName: "diffCommentBadge"
                    x: horizontalViewport.contentX + horizontalViewport.width - width - 8
                    anchors.verticalCenter: parent.verticalCenter
                    z: 8
                    width: diffRow.commentCount > 99 ? 28 : 22
                    height: 17
                    radius: 8
                    visible: diffRow.commentCount > 0
                    color: badgePointer.containsMouse ? Theme.accent : Theme.accentMuted
                    border.width: 1
                    border.color: Theme.accentStrong

                    Text {
                        anchors.centerIn: parent
                        text: diffRow.commentCount > 99 ? "99+" : diffRow.commentCount
                        color: Theme.foreground
                        font.family: Theme.monoFont
                        font.pixelSize: 8
                        font.weight: Font.Bold
                    }

                    MouseArea {
                        id: badgePointer
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            root.selectRow(diffRow.index, false)
                            root.commentsInspectorOpen = true
                        }
                    }
                }

                TapHandler {
                    objectName: "diffRowTapHandler"
                    acceptedButtons: Qt.LeftButton
                    onTapped: root.selectRow(
                        diffRow.index,
                        (point.modifiers & Qt.ShiftModifier) !== 0
                    )
                }
            }
        }
    }

    DiffCommentInspector {
        id: commentsInspector
        objectName: "diffCommentsInspector"
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        z: 40
        width: root.commentsInspectorOpen
            ? Math.min(360, Math.max(300, root.width * 0.36)) : 0
        opacity: root.commentsInspectorOpen ? 1 : 0
        visible: root.commentsInspectorOpen || width > 0
        clip: true
        backend: root.backend
        onCloseRequested: root.commentsInspectorOpen = false
        onCopyRequested: text => root.copyText(text)

        Behavior on width {
            NumberAnimation {
                duration: Theme.transitionDuration
                easing.type: Easing.OutCubic
            }
        }

        Behavior on opacity {
            NumberAnimation { duration: Theme.transitionDuration }
        }
    }

    Loader {
        id: commentComposerLoader
        objectName: "diffCommentComposerLoader"
        anchors.right: commentsInspector.left
        anchors.rightMargin: 12
        z: 30
        width: Math.max(280, Math.min(380, horizontalViewport.width - 24))
        height: active ? 126 : 0
        active: root.activeCommentRow >= 0
        y: {
            const scrollPosition = diffList.contentY
            return root.commentComposerY(scrollPosition)
        }
        sourceComponent: commentComposerComponent
    }

    Component {
        id: commentComposerComponent

        DiffCommentComposer {
            backend: root.backend
            row: root.activeCommentRow
            lineHint: root.backend.diff_comment_line_hint(root.activeCommentRow)
            onCancelled: root.closeCommentComposer()
            onSaved: {
                root.commentsInspectorOpen = true
                root.closeCommentComposer()
            }
        }
    }

    Connections {
        target: root.backend

        function onDiffCommentsStateChanged() {
            root.applyCommentTarget()
        }
    }

    TextEdit {
        id: clipboardProxy
        objectName: "diffClipboardProxy"
        x: -2
        y: -2
        width: 1
        height: 1
        opacity: 0
        readOnly: true
        textFormat: TextEdit.PlainText
    }

    Text {
        id: loadingState
        anchors.centerIn: horizontalViewport
        visible: root.backend.diffLoading
        text: "Loading diff…"
        color: Theme.muted
        font.family: Theme.uiFont
        font.pixelSize: 12
    }

    Text {
        id: errorState
        anchors.centerIn: horizontalViewport
        width: Math.min(520, horizontalViewport.width - 48)
        visible: root.backend.diffError.length > 0
        text: root.backend.diffError
        color: Theme.negative
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
        font.family: Theme.uiFont
        font.pixelSize: 12
    }

    Text {
        id: emptyState
        anchors.centerIn: horizontalViewport
        visible: !root.backend.diffLoading
            && root.backend.diffError.length === 0
            && root.backend.diffSelectedPath.length === 0
        text: "Working tree is clean"
        color: Theme.muted
        font.family: Theme.uiFont
        font.pixelSize: 12
    }

    Component.onCompleted: {
        lastCommentTargetRevision = backend.diffCommentTargetRevision
    }
}
