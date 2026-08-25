use std::collections::HashMap;

use hunk_terminal::{
    TerminalColorSnapshot, TerminalCursorShapeSnapshot, TerminalNamedColorSnapshot,
    TerminalScreenSnapshot,
};
use qtbridge::{QListModel, QListModelBase, QModelItem, qobject};

const TERMINAL_WIDE_CHAR_SPACER_FLAG: u16 = 0b0000_0000_0100_0000;
const TERMINAL_LEADING_WIDE_CHAR_SPACER_FLAG: u16 = 0b0000_0100_0000_0000;

#[derive(Clone, Debug, Default, Eq, PartialEq, QModelItem)]
pub struct TerminalTabItem {
    pub tab_id: i32,
    pub title: String,
    pub status: String,
}

#[qobject(Base = QListModel)]
mod terminal_tab_model {
    use super::{QListModel, QListModelBase, TerminalTabItem};

    #[derive(Default)]
    pub struct TerminalTabListModel {
        items: Vec<TerminalTabItem>,
        replacement: Option<Vec<TerminalTabItem>>,
    }

    impl TerminalTabListModel {
        pub fn replace(&mut self, items: Vec<TerminalTabItem>) {
            if self.items == items {
                return;
            }
            self.replacement = Some(items);
            self.reset();
        }
    }

    impl QListModel for TerminalTabListModel {
        type Item = TerminalTabItem;

        fn len(&self) -> usize {
            self.items.len()
        }

        fn get(&self, index: usize) -> Option<&Self::Item> {
            self.items.get(index)
        }

        fn set_unnotified(&mut self, index: usize, value: Self::Item) -> bool {
            let Some(item) = self.items.get_mut(index) else {
                return false;
            };
            *item = value;
            true
        }

        fn reset_unnotified(&mut self) {
            self.items = self.replacement.take().unwrap_or_default();
        }
    }
}

pub use terminal_tab_model::TerminalTabListModel;

#[derive(Clone, Debug, Default, Eq, PartialEq, QModelItem)]
pub struct TerminalRowItem {
    pub row: i32,
    pub text: String,
    pub markup: String,
}

#[qobject(Base = QListModel)]
mod terminal_row_model {
    use super::{QListModel, QListModelBase, TerminalRowItem};

    #[derive(Default)]
    pub struct TerminalRowListModel {
        items: Vec<TerminalRowItem>,
        replacement: Option<Vec<TerminalRowItem>>,
    }

    impl TerminalRowListModel {
        pub fn replace_for_tab(&mut self, items: Vec<TerminalRowItem>) -> Vec<TerminalRowItem> {
            let previous = std::mem::take(&mut self.items);
            self.replacement = Some(items);
            self.reset();
            previous
        }

        pub fn replace_or_patch(&mut self, items: Vec<TerminalRowItem>) {
            if self.items.len() != items.len() {
                self.replacement = Some(items);
                self.reset();
                return;
            }

            for (index, item) in items.into_iter().enumerate() {
                if self.items[index] != item {
                    self.set(index, item);
                }
            }
        }

        pub fn clear(&mut self) {
            if self.items.is_empty() {
                return;
            }
            self.replacement = Some(Vec::new());
            self.reset();
        }
    }

    impl QListModel for TerminalRowListModel {
        type Item = TerminalRowItem;

        fn len(&self) -> usize {
            self.items.len()
        }

        fn get(&self, index: usize) -> Option<&Self::Item> {
            self.items.get(index)
        }

        fn set_unnotified(&mut self, index: usize, value: Self::Item) -> bool {
            let Some(item) = self.items.get_mut(index) else {
                return false;
            };
            *item = value;
            true
        }

        fn reset_unnotified(&mut self) {
            self.items = self.replacement.take().unwrap_or_default();
        }
    }
}

pub use terminal_row_model::TerminalRowListModel;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TerminalScreenProjection {
    pub rows: Vec<TerminalRowItem>,
    pub first_visible_line: i32,
    pub cursor_row: i32,
    pub cursor_column: i32,
    pub cursor_shape: String,
    pub cursor_visible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProjectedCell {
    character: char,
    zerowidth: String,
    fg: TerminalColorSnapshot,
    bg: TerminalColorSnapshot,
    spacer: bool,
}

impl Default for ProjectedCell {
    fn default() -> Self {
        Self {
            character: ' ',
            zerowidth: String::new(),
            fg: TerminalColorSnapshot::Named(TerminalNamedColorSnapshot::Foreground),
            bg: TerminalColorSnapshot::Named(TerminalNamedColorSnapshot::Background),
            spacer: false,
        }
    }
}

pub fn project_terminal_screen(screen: &TerminalScreenSnapshot) -> TerminalScreenProjection {
    let (grid, first_visible_line) = projected_terminal_grid(screen);

    let projected_rows = grid
        .iter()
        .enumerate()
        .map(|(row, cells)| TerminalRowItem {
            row: saturating_usize_to_i32(row),
            text: terminal_row_text(cells),
            markup: terminal_row_markup(cells),
        })
        .collect();
    let cursor_row = screen.cursor.line.saturating_sub(first_visible_line);
    let cursor_visible = screen.mode.show_cursor
        && cursor_row >= 0
        && cursor_row < i32::from(screen.rows)
        && screen.cursor.column < usize::from(screen.cols);

    TerminalScreenProjection {
        rows: projected_rows,
        first_visible_line,
        cursor_row,
        cursor_column: saturating_usize_to_i32(screen.cursor.column),
        cursor_shape: terminal_cursor_shape(screen.cursor.shape).to_owned(),
        cursor_visible,
    }
}

pub fn terminal_selection_text(
    screen: &TerminalScreenSnapshot,
    anchor_row: i32,
    anchor_column: i32,
    head_row: i32,
    head_column: i32,
) -> String {
    let (grid, _) = projected_terminal_grid(screen);
    if grid.is_empty() {
        return String::new();
    }
    let max_row = saturating_usize_to_i32(grid.len().saturating_sub(1));
    let max_column = i32::from(screen.cols.saturating_sub(1));
    let anchor = (
        anchor_row.clamp(0, max_row),
        anchor_column.clamp(0, max_column),
    );
    let head = (head_row.clamp(0, max_row), head_column.clamp(0, max_column));
    let (start, end) = if anchor <= head {
        (anchor, head)
    } else {
        (head, anchor)
    };
    if start == end {
        return String::new();
    }

    let mut selected = Vec::new();
    for row in start.0..=end.0 {
        let Some(cells) = grid.get(row as usize) else {
            continue;
        };
        let first = if row == start.0 { start.1 } else { 0 };
        let last = if row == end.0 { end.1 } else { max_column };
        let first = usize::try_from(first).unwrap_or_default().min(cells.len());
        let last = usize::try_from(last)
            .unwrap_or_default()
            .saturating_add(1)
            .min(cells.len());
        let mut line = terminal_row_text(&cells[first..last]);
        if row != end.0 || end.1 == max_column {
            line.truncate(line.trim_end_matches(' ').len());
        }
        selected.push(line);
    }
    selected.join("\n")
}

fn projected_terminal_grid(screen: &TerminalScreenSnapshot) -> (Vec<Vec<ProjectedCell>>, i32) {
    let rows = usize::from(screen.rows.max(1));
    let cols = usize::from(screen.cols.max(1));
    let first_visible_line = screen
        .cells
        .iter()
        .map(|cell| cell.line)
        .min()
        .unwrap_or(screen.cursor.line.max(0));
    let mut grid = vec![vec![ProjectedCell::default(); cols]; rows];

    for cell in &screen.cells {
        let relative_line = cell.line - first_visible_line;
        let Ok(row) = usize::try_from(relative_line) else {
            continue;
        };
        if row >= rows || cell.column >= cols {
            continue;
        }
        if terminal_cell_is_wide_spacer(cell.flags) {
            grid[row][cell.column] = ProjectedCell {
                fg: cell.fg,
                bg: cell.bg,
                spacer: true,
                ..ProjectedCell::default()
            };
            continue;
        }
        grid[row][cell.column] = ProjectedCell {
            character: terminal_character(cell.character),
            zerowidth: cell.zerowidth.iter().collect(),
            fg: cell.fg,
            bg: cell.bg,
            spacer: false,
        };
    }

    (grid, first_visible_line)
}

fn terminal_row_text(cells: &[ProjectedCell]) -> String {
    let mut text = String::with_capacity(cells.len());
    for cell in cells {
        if cell.spacer {
            continue;
        }
        text.push(cell.character);
        text.extend(cell.zerowidth.chars().map(terminal_character));
    }
    text
}

fn terminal_row_markup(cells: &[ProjectedCell]) -> String {
    let mut markup = String::new();
    let mut start = 0usize;
    while start < cells.len() {
        let style = (cells[start].fg, cells[start].bg);
        let mut end = start + 1;
        while end < cells.len() && (cells[end].fg, cells[end].bg) == style {
            end += 1;
        }

        markup.push_str("<span style=\"color:");
        markup.push_str(terminal_color_markup(style.0).as_str());
        markup.push_str(";background-color:");
        markup.push_str(terminal_color_markup(style.1).as_str());
        markup.push_str("\">");
        for cell in &cells[start..end] {
            if cell.spacer {
                continue;
            }
            push_terminal_html_character(&mut markup, cell.character);
            for character in cell.zerowidth.chars() {
                push_terminal_html_character(&mut markup, terminal_character(character));
            }
        }
        markup.push_str("</span>");
        start = end;
    }
    markup
}

fn terminal_color_markup(color: TerminalColorSnapshot) -> String {
    match color {
        TerminalColorSnapshot::Named(named) => {
            format!("@{}@", terminal_named_color_token(named))
        }
        TerminalColorSnapshot::Indexed(index) if index < 16 => {
            format!("@{}@", terminal_indexed_color_token(index))
        }
        TerminalColorSnapshot::Indexed(index) => {
            let (r, g, b) = terminal_extended_indexed_rgb(index);
            format!("#{r:02x}{g:02x}{b:02x}")
        }
        TerminalColorSnapshot::Rgb { r, g, b } => format!("#{r:02x}{g:02x}{b:02x}"),
    }
}

fn terminal_named_color_token(color: TerminalNamedColorSnapshot) -> &'static str {
    match color {
        TerminalNamedColorSnapshot::Black => "terminalBlack",
        TerminalNamedColorSnapshot::Red => "terminalRed",
        TerminalNamedColorSnapshot::Green => "terminalGreen",
        TerminalNamedColorSnapshot::Yellow => "terminalYellow",
        TerminalNamedColorSnapshot::Blue => "terminalBlue",
        TerminalNamedColorSnapshot::Magenta => "terminalMagenta",
        TerminalNamedColorSnapshot::Cyan => "terminalCyan",
        TerminalNamedColorSnapshot::White => "terminalWhite",
        TerminalNamedColorSnapshot::BrightBlack | TerminalNamedColorSnapshot::DimBlack => {
            "terminalBrightBlack"
        }
        TerminalNamedColorSnapshot::BrightRed => "terminalBrightRed",
        TerminalNamedColorSnapshot::BrightGreen => "terminalBrightGreen",
        TerminalNamedColorSnapshot::BrightYellow => "terminalBrightYellow",
        TerminalNamedColorSnapshot::BrightBlue => "terminalBrightBlue",
        TerminalNamedColorSnapshot::BrightMagenta => "terminalBrightMagenta",
        TerminalNamedColorSnapshot::BrightCyan => "terminalBrightCyan",
        TerminalNamedColorSnapshot::BrightWhite => "terminalBrightWhite",
        TerminalNamedColorSnapshot::Foreground | TerminalNamedColorSnapshot::BrightForeground => {
            "terminalForeground"
        }
        TerminalNamedColorSnapshot::Background => "terminalBackground",
        TerminalNamedColorSnapshot::Cursor => "terminalCursor",
        TerminalNamedColorSnapshot::DimRed => "terminalDimRed",
        TerminalNamedColorSnapshot::DimGreen => "terminalDimGreen",
        TerminalNamedColorSnapshot::DimYellow => "terminalDimYellow",
        TerminalNamedColorSnapshot::DimBlue => "terminalDimBlue",
        TerminalNamedColorSnapshot::DimMagenta => "terminalDimMagenta",
        TerminalNamedColorSnapshot::DimCyan => "terminalDimCyan",
        TerminalNamedColorSnapshot::DimWhite => "terminalDimWhite",
        TerminalNamedColorSnapshot::DimForeground => "terminalDimForeground",
    }
}

fn terminal_indexed_color_token(index: u8) -> &'static str {
    match index {
        0 => "terminalBlack",
        1 => "terminalRed",
        2 => "terminalGreen",
        3 => "terminalYellow",
        4 => "terminalBlue",
        5 => "terminalMagenta",
        6 => "terminalCyan",
        7 => "terminalWhite",
        8 => "terminalBrightBlack",
        9 => "terminalBrightRed",
        10 => "terminalBrightGreen",
        11 => "terminalBrightYellow",
        12 => "terminalBrightBlue",
        13 => "terminalBrightMagenta",
        14 => "terminalBrightCyan",
        _ => "terminalBrightWhite",
    }
}

fn terminal_extended_indexed_rgb(index: u8) -> (u8, u8, u8) {
    if index >= 232 {
        let value = 8u8.saturating_add(index.saturating_sub(232).saturating_mul(10));
        return (value, value, value);
    }
    let offset = index.saturating_sub(16);
    let component = |value: u8| if value == 0 { 0 } else { 55 + value * 40 };
    (
        component(offset / 36),
        component((offset % 36) / 6),
        component(offset % 6),
    )
}

fn push_terminal_html_character(output: &mut String, character: char) {
    match character {
        '&' => output.push_str("&amp;"),
        '<' => output.push_str("&lt;"),
        '>' => output.push_str("&gt;"),
        '"' => output.push_str("&quot;"),
        '\'' => output.push_str("&#39;"),
        '@' => output.push_str("&#64;"),
        ' ' => output.push_str("&nbsp;"),
        _ => output.push(character),
    }
}

fn terminal_character(character: char) -> char {
    if character == '\0' || character.is_control() {
        ' '
    } else {
        character
    }
}

fn terminal_cell_is_wide_spacer(flags: u16) -> bool {
    flags & (TERMINAL_WIDE_CHAR_SPACER_FLAG | TERMINAL_LEADING_WIDE_CHAR_SPACER_FLAG) != 0
}

fn terminal_cursor_shape(shape: TerminalCursorShapeSnapshot) -> &'static str {
    match shape {
        TerminalCursorShapeSnapshot::Hidden => "hidden",
        TerminalCursorShapeSnapshot::Block => "block",
        TerminalCursorShapeSnapshot::Underline => "underline",
        TerminalCursorShapeSnapshot::Beam => "beam",
        TerminalCursorShapeSnapshot::HollowBlock => "hollow",
    }
}

fn saturating_usize_to_i32(value: usize) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
