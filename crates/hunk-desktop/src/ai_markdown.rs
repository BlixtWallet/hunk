use std::collections::{HashMap, HashSet};

use hunk_domain::markdown_preview::{
    MarkdownCodeSpan, MarkdownCodeTokenKind, MarkdownInlineSpan, MarkdownPreviewBlock,
    parse_markdown_preview,
};

pub(crate) const AI_MARKDOWN_MAX_VISIBLE_MESSAGES: usize = 80;
const AI_MARKDOWN_MAX_BLOCKS: usize = 128;
const AI_MARKDOWN_MAX_CODE_BLOCKS: usize = 32;
const AI_MARKDOWN_MAX_SPANS: usize = 2_048;
const AI_MARKDOWN_MAX_PROJECTED_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AiMarkdownBlockProjection {
    pub kind: String,
    pub text: String,
    pub markup: String,
    pub language: String,
    pub heading_level: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CachedMarkdownProjection {
    last_sequence: i64,
    text: String,
    blocks: Option<Vec<AiMarkdownBlockProjection>>,
}

#[derive(Default)]
pub(crate) struct AiMarkdownProjectionCache {
    entries: HashMap<String, CachedMarkdownProjection>,
}

impl AiMarkdownProjectionCache {
    pub(crate) fn project_completed_message(
        &mut self,
        row_id: &str,
        last_sequence: i64,
        text: &str,
    ) -> Option<Vec<AiMarkdownBlockProjection>> {
        if text.trim().is_empty() {
            self.entries.remove(row_id);
            return None;
        }
        if let Some(cached) = self.entries.get(row_id)
            && cached.last_sequence == last_sequence
            && cached.text == text
        {
            return cached.blocks.clone();
        }

        let blocks = markdown_blocks(text);
        self.entries.insert(
            row_id.to_owned(),
            CachedMarkdownProjection {
                last_sequence,
                text: text.to_owned(),
                blocks: blocks.clone(),
            },
        );
        blocks
    }

    pub(crate) fn remove(&mut self, row_id: &str) {
        self.entries.remove(row_id);
    }

    pub(crate) fn retain_visible<'a>(&mut self, row_ids: impl IntoIterator<Item = &'a str>) {
        let visible = row_ids.into_iter().collect::<HashSet<_>>();
        self.entries
            .retain(|row_id, _| visible.contains(row_id.as_str()));
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }
}

fn markdown_blocks(markdown: &str) -> Option<Vec<AiMarkdownBlockProjection>> {
    let blocks = parse_markdown_preview(markdown);
    if blocks.is_empty() || blocks.len() > AI_MARKDOWN_MAX_BLOCKS {
        return None;
    }

    let mut code_block_count = 0usize;
    let mut span_count = 0usize;
    for block in &blocks {
        match block {
            MarkdownPreviewBlock::Heading { spans, .. }
            | MarkdownPreviewBlock::Paragraph(spans)
            | MarkdownPreviewBlock::UnorderedListItem(spans)
            | MarkdownPreviewBlock::OrderedListItem { spans, .. }
            | MarkdownPreviewBlock::BlockQuote(spans) => {
                span_count = span_count.saturating_add(spans.len());
            }
            MarkdownPreviewBlock::CodeBlock { lines, .. } => {
                code_block_count = code_block_count.saturating_add(1);
                span_count = span_count.saturating_add(lines.iter().map(Vec::len).sum::<usize>());
            }
            MarkdownPreviewBlock::ThematicBreak => {}
        }
    }
    if code_block_count > AI_MARKDOWN_MAX_CODE_BLOCKS || span_count > AI_MARKDOWN_MAX_SPANS {
        return None;
    }

    let projected = blocks
        .into_iter()
        .map(markdown_block_projection)
        .collect::<Vec<_>>();
    let projected_bytes = projected.iter().fold(0usize, |total, block| {
        total
            .saturating_add(block.text.len())
            .saturating_add(block.markup.len())
            .saturating_add(block.language.len())
    });
    (projected_bytes <= AI_MARKDOWN_MAX_PROJECTED_BYTES).then_some(projected)
}

fn markdown_block_projection(block: MarkdownPreviewBlock) -> AiMarkdownBlockProjection {
    match block {
        MarkdownPreviewBlock::Heading { level, spans } => {
            let (text, markup) = inline_text_and_markup(spans.as_slice());
            projected_block("heading", text, markup, String::new(), i32::from(level))
        }
        MarkdownPreviewBlock::Paragraph(spans) => {
            let (text, markup) = inline_text_and_markup(spans.as_slice());
            projected_block("paragraph", text, markup, String::new(), 0)
        }
        MarkdownPreviewBlock::UnorderedListItem(spans) => {
            let (text, markup) = prefixed_inline_text_and_markup("• ", spans.as_slice());
            projected_block("list", text, markup, String::new(), 0)
        }
        MarkdownPreviewBlock::OrderedListItem { number, spans } => {
            let prefix = format!("{number}. ");
            let (text, markup) = prefixed_inline_text_and_markup(prefix.as_str(), spans.as_slice());
            projected_block("list", text, markup, String::new(), 0)
        }
        MarkdownPreviewBlock::BlockQuote(spans) => {
            let (text, markup) = inline_text_and_markup(spans.as_slice());
            projected_block("quote", text, markup, String::new(), 0)
        }
        MarkdownPreviewBlock::CodeBlock { language, lines } => {
            let (text, markup) = code_text_and_markup(lines.as_slice());
            projected_block("code", text, markup, language.unwrap_or_default(), 0)
        }
        MarkdownPreviewBlock::ThematicBreak => {
            projected_block("rule", String::new(), String::new(), String::new(), 0)
        }
    }
}

fn projected_block(
    kind: &str,
    text: String,
    markup: String,
    language: String,
    heading_level: i32,
) -> AiMarkdownBlockProjection {
    AiMarkdownBlockProjection {
        kind: kind.to_owned(),
        text,
        markup,
        language,
        heading_level,
    }
}

fn prefixed_inline_text_and_markup(prefix: &str, spans: &[MarkdownInlineSpan]) -> (String, String) {
    let (text, markup) = inline_text_and_markup(spans);
    let mut prefixed_markup = String::from("<font color=\"@plain@\">");
    push_html_escaped(&mut prefixed_markup, prefix, false);
    prefixed_markup.push_str("</font>");
    prefixed_markup.push_str(markup.as_str());
    (format!("{prefix}{text}"), prefixed_markup)
}

fn inline_text_and_markup(spans: &[MarkdownInlineSpan]) -> (String, String) {
    let mut text = String::new();
    let mut markup = String::new();

    for span in spans {
        if span.style.hard_break {
            text.push('\n');
            markup.push_str("<br>");
            continue;
        }
        if span.text.is_empty() {
            continue;
        }

        text.push_str(span.text.as_str());
        let color = if span.style.link.is_some() {
            "link"
        } else if span.style.code {
            "constant"
        } else {
            "plain"
        };
        markup.push_str("<font color=\"@");
        markup.push_str(color);
        markup.push_str("@\">");
        if span.style.bold {
            markup.push_str("<b>");
        }
        if span.style.italic {
            markup.push_str("<i>");
        }
        if span.style.strikethrough {
            markup.push_str("<s>");
        }
        if span.style.link.is_some() {
            markup.push_str("<u>");
        }
        push_html_escaped(&mut markup, span.text.as_str(), span.style.code);
        if span.style.link.is_some() {
            markup.push_str("</u>");
        }
        if span.style.strikethrough {
            markup.push_str("</s>");
        }
        if span.style.italic {
            markup.push_str("</i>");
        }
        if span.style.bold {
            markup.push_str("</b>");
        }
        markup.push_str("</font>");
    }

    (text, markup)
}

fn code_text_and_markup(lines: &[Vec<MarkdownCodeSpan>]) -> (String, String) {
    let mut text = String::new();
    let mut markup = String::new();

    for (line_index, line) in lines.iter().enumerate() {
        if line_index > 0 {
            text.push('\n');
            markup.push_str("<br>");
        }
        for span in line {
            text.push_str(span.text.as_str());
            markup.push_str("<font color=\"@");
            markup.push_str(code_token_name(span.token));
            markup.push_str("@\">");
            push_html_escaped(&mut markup, span.text.as_str(), true);
            markup.push_str("</font>");
        }
    }

    (text, markup)
}

fn code_token_name(token: MarkdownCodeTokenKind) -> &'static str {
    match token {
        MarkdownCodeTokenKind::Plain => "plain",
        MarkdownCodeTokenKind::Keyword => "keyword",
        MarkdownCodeTokenKind::String => "string",
        MarkdownCodeTokenKind::Number => "number",
        MarkdownCodeTokenKind::Comment => "comment",
        MarkdownCodeTokenKind::Function => "function",
        MarkdownCodeTokenKind::TypeName => "type",
        MarkdownCodeTokenKind::Constant => "constant",
        MarkdownCodeTokenKind::Variable => "variable",
        MarkdownCodeTokenKind::Operator => "operator",
    }
}

fn push_html_escaped(output: &mut String, text: &str, preserve_whitespace: bool) {
    for character in text.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            '@' => output.push_str("&#64;"),
            ' ' if preserve_whitespace => output.push_str("&nbsp;"),
            '\t' if preserve_whitespace => output.push_str("&nbsp;&nbsp;&nbsp;&nbsp;"),
            _ => output.push(character),
        }
    }
}
