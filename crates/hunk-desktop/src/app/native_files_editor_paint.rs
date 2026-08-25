use gpui::{App, Font, Hsla, Pixels, Point, ShapedLine, SharedString, TextAlign, TextRun, Window};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RowSyntaxSpan {
    pub(crate) start_column: usize,
    pub(crate) end_column: usize,
    pub(crate) style_key: String,
}

pub(crate) fn single_color_text_run(len: usize, color: Hsla, font: Font) -> TextRun {
    TextRun {
        len,
        color,
        font,
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}

pub(crate) fn shape_editor_line(
    window: &mut Window,
    text: SharedString,
    font_size: Pixels,
    runs: &[TextRun],
) -> ShapedLine {
    window.text_system().shape_line(text, font_size, runs, None)
}

pub(crate) fn paint_editor_line(
    window: &mut Window,
    cx: &mut App,
    line: &ShapedLine,
    origin: Point<Pixels>,
    line_height: Pixels,
) {
    let _ = line.paint(origin, line_height, TextAlign::Left, None, window, cx);
}
