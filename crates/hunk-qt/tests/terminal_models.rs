use hunk_qt::{
    TerminalRowItem, TerminalRowListModel, TerminalTabItem, TerminalTabListModel,
    project_terminal_screen, terminal_selection_text,
};
use hunk_terminal::{
    TerminalCellSnapshot, TerminalColorSnapshot, TerminalCursorShapeSnapshot,
    TerminalCursorSnapshot, TerminalDamageSnapshot, TerminalModeSnapshot,
    TerminalNamedColorSnapshot, TerminalScreenSnapshot,
};
use qtbridge::{QListModel, QObjectHolder};

const WIDE_CHAR_FLAG: u16 = 0b0000_0000_0010_0000;
const WIDE_CHAR_SPACER_FLAG: u16 = 0b0000_0000_0100_0000;

fn cell(line: i32, column: usize, character: char) -> TerminalCellSnapshot {
    TerminalCellSnapshot {
        line,
        column,
        character,
        fg: TerminalColorSnapshot::Named(TerminalNamedColorSnapshot::Foreground),
        bg: TerminalColorSnapshot::Named(TerminalNamedColorSnapshot::Background),
        flags: 0,
        zerowidth: Vec::new(),
    }
}

fn screen(rows: u16, cols: u16, cells: Vec<TerminalCellSnapshot>) -> TerminalScreenSnapshot {
    TerminalScreenSnapshot {
        rows,
        cols,
        display_offset: 0,
        cursor: TerminalCursorSnapshot {
            line: 0,
            column: 0,
            shape: TerminalCursorShapeSnapshot::Block,
        },
        mode: TerminalModeSnapshot {
            show_cursor: true,
            ..TerminalModeSnapshot::default()
        },
        damage: TerminalDamageSnapshot::Full,
        cells,
    }
}

#[test]
fn projection_preserves_terminal_rows_colors_and_cursor_coordinates() {
    let mut less_than = cell(-1, 0, '<');
    less_than.fg = TerminalColorSnapshot::Named(TerminalNamedColorSnapshot::Red);
    let at = cell(-1, 1, '@');
    let mut indexed = cell(0, 0, 'x');
    indexed.fg = TerminalColorSnapshot::Indexed(196);
    let mut snapshot = screen(2, 4, vec![less_than, at, indexed]);
    snapshot.display_offset = 1;
    snapshot.cursor = TerminalCursorSnapshot {
        line: 0,
        column: 2,
        shape: TerminalCursorShapeSnapshot::Beam,
    };

    let projection = project_terminal_screen(&snapshot);

    assert_eq!(projection.first_visible_line, -1);
    assert_eq!(projection.rows.len(), 2);
    assert_eq!(projection.rows[0].text, "<@  ");
    assert!(projection.rows[0].markup.contains("@terminalRed@"));
    assert!(projection.rows[0].markup.contains("&lt;"));
    assert!(projection.rows[0].markup.contains("&#64;"));
    assert!(projection.rows[0].markup.contains("&nbsp;"));
    assert!(projection.rows[1].markup.contains("#ff0000"));
    assert_eq!(projection.cursor_row, 1);
    assert_eq!(projection.cursor_column, 2);
    assert_eq!(projection.cursor_shape, "beam");
    assert!(projection.cursor_visible);
}

#[test]
fn projection_and_selection_keep_wide_and_combining_cells_aligned() {
    let mut wide = cell(0, 0, '界');
    wide.flags = WIDE_CHAR_FLAG;
    let mut spacer = cell(0, 1, ' ');
    spacer.flags = WIDE_CHAR_SPACER_FLAG;
    let mut combining = cell(0, 2, 'e');
    combining.zerowidth.push('\u{301}');
    let snapshot = screen(1, 4, vec![wide, spacer, combining]);

    let projection = project_terminal_screen(&snapshot);

    assert_eq!(projection.rows[0].text, "界e\u{301} ");
    assert_eq!(terminal_selection_text(&snapshot, 0, 0, 0, 2), "界e\u{301}");
    assert_eq!(terminal_selection_text(&snapshot, 0, 2, 0, 0), "界e\u{301}");
}

#[test]
fn multi_row_selection_trims_only_completed_line_endings() {
    let snapshot = screen(
        2,
        5,
        vec![cell(0, 0, 'a'), cell(0, 1, 'b'), cell(1, 0, 'c')],
    );

    assert_eq!(terminal_selection_text(&snapshot, 0, 1, 1, 1), "b\nc ");
    assert!(terminal_selection_text(&snapshot, 1, 1, 1, 1).is_empty());
}

#[test]
fn terminal_models_replace_rows_and_tabs_without_stale_items() {
    let rows = TerminalRowListModel::default_with_attached_qobject();
    let mut rows = rows.borrow_mut();
    rows.replace_or_patch(vec![TerminalRowItem {
        row: 0,
        text: "old".to_owned(),
        markup: "old".to_owned(),
    }]);
    rows.replace_or_patch(vec![TerminalRowItem {
        row: 0,
        text: "new".to_owned(),
        markup: "new".to_owned(),
    }]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows.get(0).expect("updated terminal row").text, "new");
    let previous = rows.replace_for_tab(vec![TerminalRowItem {
        row: 0,
        text: "other tab".to_owned(),
        markup: "other tab".to_owned(),
    }]);
    assert_eq!(previous[0].text, "new");
    assert_eq!(
        rows.get(0).expect("replacement terminal row").text,
        "other tab"
    );
    rows.clear();
    assert_eq!(rows.len(), 0);

    let tabs = TerminalTabListModel::default_with_attached_qobject();
    let mut tabs = tabs.borrow_mut();
    tabs.replace(vec![TerminalTabItem {
        tab_id: 7,
        title: "zsh".to_owned(),
        status: "running".to_owned(),
    }]);
    assert_eq!(tabs.len(), 1);
    assert_eq!(tabs.get(0).expect("terminal tab").tab_id, 7);
}
