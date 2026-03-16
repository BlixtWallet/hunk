use std::sync::OnceLock;

use comrak::nodes::{ListType, NodeCodeBlock, NodeLink, NodeList, NodeValue};
use syntect::easy::ScopeRegionIterator;
use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

type ComrakNode<'a> = comrak::nodes::Node<'a>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkdownPreviewBlock {
    Heading {
        level: u8,
        spans: Vec<MarkdownInlineSpan>,
    },
    Paragraph(Vec<MarkdownInlineSpan>),
    UnorderedListItem(Vec<MarkdownInlineSpan>),
    OrderedListItem {
        number: usize,
        spans: Vec<MarkdownInlineSpan>,
    },
    BlockQuote(Vec<MarkdownInlineSpan>),
    CodeBlock {
        language: Option<String>,
        lines: Vec<Vec<MarkdownCodeSpan>>,
    },
    ThematicBreak,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MarkdownInlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,
    pub hard_break: bool,
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownInlineSpan {
    pub text: String,
    pub style: MarkdownInlineStyle,
}

impl MarkdownInlineSpan {
    fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: MarkdownInlineStyle::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkdownCodeTokenKind {
    Plain,
    Keyword,
    String,
    Number,
    Comment,
    Function,
    TypeName,
    Constant,
    Variable,
    Operator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownCodeSpan {
    pub text: String,
    pub token: MarkdownCodeTokenKind,
}

pub fn parse_markdown_preview(markdown: &str) -> Vec<MarkdownPreviewBlock> {
    if markdown.trim().is_empty() {
        return Vec::new();
    }

    let arena = comrak::Arena::new();
    let options = markdown_parse_options();
    let root = comrak::parse_document(&arena, markdown, &options);

    let mut blocks = Vec::new();
    for node in root.children() {
        parse_flow_node(node, &mut blocks);
    }
    blocks
}

fn markdown_parse_options() -> comrak::Options<'static> {
    let mut options = comrak::Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options
}

fn parse_flow_node(node: ComrakNode<'_>, out: &mut Vec<MarkdownPreviewBlock>) {
    let data = node.data();
    match &data.value {
        NodeValue::Heading(heading) => {
            let spans = parse_inline_nodes(node);
            if !spans.is_empty() {
                out.push(MarkdownPreviewBlock::Heading {
                    level: heading.level,
                    spans,
                });
            }
        }
        NodeValue::Paragraph => {
            let spans = parse_inline_nodes(node);
            if !spans.is_empty() {
                out.push(MarkdownPreviewBlock::Paragraph(spans));
            }
        }
        NodeValue::List(list) => {
            let list = *list;
            drop(data);
            parse_list_block(node, list, out);
        }
        NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) | NodeValue::Alert(_) => {
            let spans = parse_container_nodes_as_inline(node);
            if !spans.is_empty() {
                out.push(MarkdownPreviewBlock::BlockQuote(spans));
            }
        }
        NodeValue::CodeBlock(code) => {
            let language = code_block_language_hint(code);
            out.push(MarkdownPreviewBlock::CodeBlock {
                language: language.clone(),
                lines: highlight_code_lines(language.as_deref(), code.literal.as_str()),
            });
        }
        NodeValue::Math(math) => {
            out.push(MarkdownPreviewBlock::CodeBlock {
                language: Some("math".to_string()),
                lines: highlight_code_lines(None, math.literal.as_str()),
            });
        }
        NodeValue::ThematicBreak => out.push(MarkdownPreviewBlock::ThematicBreak),
        NodeValue::HtmlBlock(html) => {
            if !html.literal.trim().is_empty() {
                out.push(MarkdownPreviewBlock::Paragraph(vec![
                    MarkdownInlineSpan::plain(html.literal.clone()),
                ]));
            }
        }
        NodeValue::Table(..) => {
            drop(data);
            parse_table_as_blocks(node, out);
        }
        _ => {
            let spans = parse_container_nodes_as_inline(node);
            if !spans.is_empty() {
                out.push(MarkdownPreviewBlock::Paragraph(spans));
            }
        }
    }
}

fn parse_list_block(
    list_node: ComrakNode<'_>,
    list: NodeList,
    out: &mut Vec<MarkdownPreviewBlock>,
) {
    let mut number = list.start;
    for child in list_node.children() {
        let child_data = child.data();
        if !matches!(
            child_data.value,
            NodeValue::Item(..) | NodeValue::TaskItem(..)
        ) {
            continue;
        }
        drop(child_data);

        let spans = parse_container_nodes_as_inline(child);
        if spans.is_empty() {
            continue;
        }

        if list.list_type == ListType::Ordered {
            out.push(MarkdownPreviewBlock::OrderedListItem { number, spans });
            number = number.saturating_add(1);
        } else {
            out.push(MarkdownPreviewBlock::UnorderedListItem(spans));
        }
    }
}

fn code_block_language_hint(code: &NodeCodeBlock) -> Option<String> {
    code.info
        .split_whitespace()
        .next()
        .map(str::trim)
        .filter(|hint| !hint.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_table_as_blocks(node: ComrakNode<'_>, out: &mut Vec<MarkdownPreviewBlock>) {
    for row in node.children() {
        let row_data = row.data();
        if !matches!(row_data.value, NodeValue::TableRow(..)) {
            continue;
        }
        drop(row_data);

        let mut row_spans = Vec::new();
        for (cell_ix, cell) in row.children().enumerate() {
            if cell_ix > 0 {
                push_inline_span(
                    &mut row_spans,
                    " | ",
                    &MarkdownInlineStyle {
                        code: true,
                        ..MarkdownInlineStyle::default()
                    },
                );
            }

            let cell_data = cell.data();
            if !matches!(cell_data.value, NodeValue::TableCell) {
                continue;
            }
            drop(cell_data);

            let cell_spans = parse_container_nodes_as_inline(cell);
            for span in cell_spans {
                push_inline_span(&mut row_spans, span.text.as_str(), &span.style);
            }
        }

        if !row_spans.is_empty() {
            out.push(MarkdownPreviewBlock::Paragraph(row_spans));
        }
    }
}

fn parse_container_nodes_as_inline(node: ComrakNode<'_>) -> Vec<MarkdownInlineSpan> {
    let mut spans = Vec::new();
    let mut has_any = false;

    for child in node.children() {
        let child_data = child.data();
        let child_spans = match &child_data.value {
            NodeValue::Paragraph
            | NodeValue::Heading(..)
            | NodeValue::TableCell
            | NodeValue::DescriptionTerm
            | NodeValue::Subtext => {
                drop(child_data);
                parse_inline_nodes(child)
            }
            NodeValue::BlockQuote
            | NodeValue::MultilineBlockQuote(_)
            | NodeValue::Alert(_)
            | NodeValue::DescriptionDetails => {
                drop(child_data);
                parse_container_nodes_as_inline(child)
            }
            NodeValue::List(list) => {
                let list = *list;
                drop(child_data);
                list_children_as_inline(child, list)
            }
            NodeValue::CodeBlock(code) => {
                let literal = code.literal.clone();
                drop(child_data);
                vec![MarkdownInlineSpan {
                    text: literal,
                    style: MarkdownInlineStyle {
                        code: true,
                        ..MarkdownInlineStyle::default()
                    },
                }]
            }
            NodeValue::HtmlBlock(html) => {
                let literal = html.literal.clone();
                drop(child_data);
                vec![MarkdownInlineSpan::plain(literal)]
            }
            _ => {
                drop(child_data);
                parse_inline_nodes(child)
            }
        };

        if child_spans.is_empty() {
            continue;
        }
        if has_any && !spans_end_with_whitespace(&spans) {
            spans.push(MarkdownInlineSpan::plain(" "));
        }
        spans.extend(child_spans);
        has_any = true;
    }

    compact_inline_spans(spans)
}

fn list_children_as_inline(list_node: ComrakNode<'_>, list: NodeList) -> Vec<MarkdownInlineSpan> {
    let mut spans = Vec::new();
    let mut number = list.start;

    for child in list_node.children() {
        if !spans.is_empty() {
            spans.push(MarkdownInlineSpan::plain(" "));
        }

        let child_data = child.data();
        if !matches!(
            child_data.value,
            NodeValue::Item(..) | NodeValue::TaskItem(..)
        ) {
            continue;
        }
        drop(child_data);

        let marker = if list.list_type == ListType::Ordered {
            let label = format!("{number}. ");
            number = number.saturating_add(1);
            label
        } else {
            "- ".to_string()
        };
        spans.push(MarkdownInlineSpan::plain(marker));
        spans.extend(parse_container_nodes_as_inline(child));
    }

    compact_inline_spans(spans)
}

fn parse_inline_nodes(node: ComrakNode<'_>) -> Vec<MarkdownInlineSpan> {
    let mut spans = Vec::new();
    let base = MarkdownInlineStyle::default();
    for child in node.children() {
        parse_inline_node(child, &base, &mut spans);
    }
    compact_inline_spans(spans)
}

fn parse_inline_node(
    node: ComrakNode<'_>,
    style: &MarkdownInlineStyle,
    out: &mut Vec<MarkdownInlineSpan>,
) {
    let data = node.data();
    match &data.value {
        NodeValue::Text(text) => push_inline_span(out, text.as_ref(), style),
        NodeValue::Code(code) => {
            let next_style = updated_inline_style(style, |next| next.code = true);
            push_inline_span(out, code.literal.as_str(), &next_style);
        }
        NodeValue::Math(math) => {
            let next_style = updated_inline_style(style, |next| next.code = true);
            push_inline_span(out, math.literal.as_str(), &next_style);
        }
        NodeValue::Emph => {
            let next_style = updated_inline_style(style, |next| next.italic = true);
            drop(data);
            push_inline_children(node, &next_style, out);
        }
        NodeValue::Strong => {
            let next_style = updated_inline_style(style, |next| next.bold = true);
            drop(data);
            push_inline_children(node, &next_style, out);
        }
        NodeValue::Strikethrough => {
            let next_style = updated_inline_style(style, |next| next.strikethrough = true);
            drop(data);
            push_inline_children(node, &next_style, out);
        }
        NodeValue::Link(link) => {
            let next_style = updated_inline_style(style, |next| next.link = Some(link.url.clone()));
            drop(data);
            push_inline_children(node, &next_style, out);
        }
        NodeValue::WikiLink(link) => {
            let next_style = updated_inline_style(style, |next| next.link = Some(link.url.clone()));
            drop(data);
            push_inline_children(node, &next_style, out);
        }
        NodeValue::Image(image) => parse_image_inline(node, image.as_ref(), style, out),
        NodeValue::FootnoteReference(footnote_reference) => {
            push_inline_span(
                out,
                format!("[^{}]", footnote_reference.name).as_str(),
                style,
            );
        }
        NodeValue::LineBreak => push_hard_break_span(out, style),
        NodeValue::SoftBreak => push_inline_span(out, " ", style),
        NodeValue::HtmlInline(html) => push_inline_span(out, html.as_str(), style),
        NodeValue::Raw(raw) => push_inline_span(out, raw.as_str(), style),
        NodeValue::FrontMatter(front_matter) => push_inline_span(out, front_matter.as_str(), style),
        NodeValue::EscapedTag(tag) => push_inline_span(out, tag, style),
        _ => {
            drop(data);
            for child in node.children() {
                parse_inline_node(child, style, out);
            }
        }
    }
}

fn updated_inline_style(
    style: &MarkdownInlineStyle,
    update: impl FnOnce(&mut MarkdownInlineStyle),
) -> MarkdownInlineStyle {
    let mut next_style = style.clone();
    update(&mut next_style);
    next_style
}

fn parse_image_inline(
    node: ComrakNode<'_>,
    image: &NodeLink,
    style: &MarkdownInlineStyle,
    out: &mut Vec<MarkdownInlineSpan>,
) {
    let next_style = updated_inline_style(style, |next| next.link = Some(image.url.clone()));
    let mut image_spans = Vec::new();
    for child in node.children() {
        parse_inline_node(child, &next_style, &mut image_spans);
    }
    if image_spans.is_empty() {
        push_inline_span(out, "[image]", &next_style);
    } else {
        out.extend(image_spans);
    }
}

fn push_inline_children(
    node: ComrakNode<'_>,
    style: &MarkdownInlineStyle,
    out: &mut Vec<MarkdownInlineSpan>,
) {
    for child in node.children() {
        parse_inline_node(child, style, out);
    }
}

fn push_inline_span(out: &mut Vec<MarkdownInlineSpan>, text: &str, style: &MarkdownInlineStyle) {
    if text.is_empty() {
        return;
    }

    if let Some(last) = out.last_mut()
        && last.style == *style
    {
        last.text.push_str(text);
        return;
    }

    out.push(MarkdownInlineSpan {
        text: text.to_owned(),
        style: style.clone(),
    });
}

fn push_hard_break_span(out: &mut Vec<MarkdownInlineSpan>, style: &MarkdownInlineStyle) {
    let next_style = updated_inline_style(style, |next| next.hard_break = true);
    out.push(MarkdownInlineSpan {
        text: String::new(),
        style: next_style,
    });
}

fn compact_inline_spans(spans: Vec<MarkdownInlineSpan>) -> Vec<MarkdownInlineSpan> {
    let mut compacted: Vec<MarkdownInlineSpan> = Vec::with_capacity(spans.len());
    for span in spans {
        if span.style.hard_break {
            compacted.push(span);
            continue;
        }
        if span.text.is_empty() {
            continue;
        }
        if let Some(last) = compacted.last_mut()
            && last.style == span.style
        {
            last.text.push_str(span.text.as_str());
            continue;
        }
        compacted.push(span);
    }
    compacted
}

fn spans_end_with_whitespace(spans: &[MarkdownInlineSpan]) -> bool {
    if spans.last().is_some_and(|span| span.style.hard_break) {
        return true;
    }
    spans
        .last()
        .and_then(|span| span.text.chars().last())
        .is_some_and(char::is_whitespace)
}

fn highlight_code_lines(language: Option<&str>, code: &str) -> Vec<Vec<MarkdownCodeSpan>> {
    let syntax_set = syntax_set();
    let syntax = syntax_for_language(syntax_set, language);
    let mut rows = Vec::new();

    match syntax {
        Some(syntax) => {
            let mut parse_state = ParseState::new(syntax);
            let mut scope_stack = ScopeStack::new();
            for line in code.lines() {
                rows.push(highlight_code_line(
                    line,
                    syntax_set,
                    &mut parse_state,
                    &mut scope_stack,
                ));
            }
        }
        None => {
            for line in code.lines() {
                rows.push(vec![MarkdownCodeSpan {
                    text: line.to_owned(),
                    token: MarkdownCodeTokenKind::Plain,
                }]);
            }
        }
    }

    if rows.is_empty() {
        rows.push(vec![MarkdownCodeSpan {
            text: String::new(),
            token: MarkdownCodeTokenKind::Plain,
        }]);
    }

    rows
}

fn highlight_code_line(
    line: &str,
    syntax_set: &SyntaxSet,
    parse_state: &mut ParseState,
    scope_stack: &mut ScopeStack,
) -> Vec<MarkdownCodeSpan> {
    let chars = line.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return vec![MarkdownCodeSpan {
            text: String::new(),
            token: MarkdownCodeTokenKind::Plain,
        }];
    }

    let mut token_map = vec![MarkdownCodeTokenKind::Plain; chars.len()];
    let Ok(ops) = parse_state.parse_line(line, syntax_set) else {
        return vec![MarkdownCodeSpan {
            text: line.to_owned(),
            token: MarkdownCodeTokenKind::Plain,
        }];
    };

    let mut start = 0usize;
    for (region, op) in ScopeRegionIterator::new(&ops, line) {
        let end = (start + region.chars().count()).min(token_map.len());
        let token = if scope_stack.apply(op).is_ok() {
            syntax_token_from_scope_stack(scope_stack)
        } else {
            MarkdownCodeTokenKind::Plain
        };
        for kind in token_map.iter_mut().take(end).skip(start) {
            *kind = token;
        }
        start = end;
        if start >= token_map.len() {
            break;
        }
    }

    merge_code_spans(&chars, &token_map)
}

fn merge_code_spans(chars: &[char], token_map: &[MarkdownCodeTokenKind]) -> Vec<MarkdownCodeSpan> {
    if chars.is_empty() {
        return vec![MarkdownCodeSpan {
            text: String::new(),
            token: MarkdownCodeTokenKind::Plain,
        }];
    }

    let mut spans = Vec::new();
    let mut start = 0usize;
    let mut current = token_map[0];
    for index in 1..=chars.len() {
        let boundary = index == chars.len() || token_map[index] != current;
        if !boundary {
            continue;
        }
        spans.push(MarkdownCodeSpan {
            text: chars[start..index].iter().collect::<String>(),
            token: current,
        });
        if index < chars.len() {
            start = index;
            current = token_map[index];
        }
    }
    spans
}

fn syntax_set() -> &'static SyntaxSet {
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_nonewlines)
}

fn syntax_for_language<'a>(
    syntax_set: &'a SyntaxSet,
    language: Option<&str>,
) -> Option<&'a SyntaxReference> {
    let hint = language?.trim();
    if hint.is_empty() {
        return None;
    }

    let lower = hint.to_ascii_lowercase();
    if let Some(syntax) = syntax_set.find_syntax_by_token(lower.as_str()) {
        return Some(syntax);
    }
    if let Some(syntax) = syntax_set.find_syntax_by_extension(lower.as_str()) {
        return Some(syntax);
    }
    if let Some(tokens) = language_tokens_for_hint(lower.as_str())
        && let Some(syntax) = find_first_syntax_by_tokens(syntax_set, tokens)
    {
        return Some(syntax);
    }

    None
}

fn find_first_syntax_by_tokens<'a>(
    syntax_set: &'a SyntaxSet,
    tokens: &[&str],
) -> Option<&'a SyntaxReference> {
    tokens
        .iter()
        .find_map(|token| syntax_set.find_syntax_by_token(token))
}

fn language_tokens_for_hint(hint: &str) -> Option<&'static [&'static str]> {
    match hint {
        "js" | "jsx" | "javascript" => Some(&["js", "javascript"]),
        "ts" | "tsx" | "typescript" => Some(&["ts", "typescript", "js"]),
        "rs" | "rust" => Some(&["rs", "rust"]),
        "py" | "python" => Some(&["py", "python"]),
        "go" => Some(&["go"]),
        "json" | "jsonc" => Some(&["json", "js"]),
        "yml" | "yaml" => Some(&["yaml", "yml"]),
        "toml" => Some(&["toml"]),
        "bash" | "sh" | "zsh" | "shell" => Some(&["bash", "sh"]),
        "c" | "h" => Some(&["c", "cpp"]),
        "cc" | "cpp" | "cxx" | "hpp" | "hxx" | "c++" => Some(&["cpp", "c++", "c"]),
        "java" => Some(&["java"]),
        "kotlin" | "kt" | "kts" => Some(&["kotlin", "java"]),
        "swift" => Some(&["swift"]),
        "markdown" | "md" => Some(&["markdown", "md"]),
        _ => None,
    }
}

fn syntax_token_from_scope_stack(scope_stack: &ScopeStack) -> MarkdownCodeTokenKind {
    for scope in scope_stack.as_slice().iter().rev() {
        let scope_name = scope.build_string();
        if is_comment_scope(&scope_name) {
            return MarkdownCodeTokenKind::Comment;
        }
        if is_string_scope(&scope_name) {
            return MarkdownCodeTokenKind::String;
        }
        if is_number_scope(&scope_name) {
            return MarkdownCodeTokenKind::Number;
        }
        if is_function_scope(&scope_name) {
            return MarkdownCodeTokenKind::Function;
        }
        if is_type_scope(&scope_name) {
            return MarkdownCodeTokenKind::TypeName;
        }
        if is_constant_scope(&scope_name) {
            return MarkdownCodeTokenKind::Constant;
        }
        if is_keyword_scope(&scope_name) {
            return MarkdownCodeTokenKind::Keyword;
        }
        if is_variable_scope(&scope_name) {
            return MarkdownCodeTokenKind::Variable;
        }
        if is_operator_scope(&scope_name) {
            return MarkdownCodeTokenKind::Operator;
        }
    }
    MarkdownCodeTokenKind::Plain
}

fn is_comment_scope(scope_name: &str) -> bool {
    scope_name.starts_with("comment")
        || scope_name.contains(".comment.")
        || scope_name.ends_with(".comment")
}

fn is_string_scope(scope_name: &str) -> bool {
    scope_name.starts_with("string")
        || scope_name.contains(".string.")
        || scope_name.ends_with(".string")
}

fn is_number_scope(scope_name: &str) -> bool {
    scope_name.starts_with("constant.numeric")
        || scope_name.contains(".constant.numeric.")
        || scope_name.contains(".number.")
        || scope_name.ends_with(".number")
        || scope_name.ends_with(".numeric")
}

fn is_function_scope(scope_name: &str) -> bool {
    scope_name.starts_with("entity.name.function")
        || scope_name.contains(".entity.name.function.")
        || scope_name.starts_with("support.function")
        || scope_name.contains(".support.function.")
        || scope_name.starts_with("variable.function")
        || scope_name.contains(".variable.function.")
        || scope_name.starts_with("meta.function")
}

fn is_type_scope(scope_name: &str) -> bool {
    scope_name.starts_with("entity.name.type")
        || scope_name.contains(".entity.name.type.")
        || scope_name.starts_with("entity.name.class")
        || scope_name.contains(".entity.name.class.")
        || scope_name.starts_with("support.type")
        || scope_name.contains(".support.type.")
        || scope_name.starts_with("storage.type")
        || scope_name.contains(".storage.type.")
}

fn is_constant_scope(scope_name: &str) -> bool {
    scope_name.starts_with("constant")
        || scope_name.contains(".constant.")
        || scope_name.ends_with(".constant")
}

fn is_keyword_scope(scope_name: &str) -> bool {
    scope_name.starts_with("keyword")
        || scope_name.contains(".keyword.")
        || scope_name.ends_with(".keyword")
        || scope_name.starts_with("storage.modifier")
        || scope_name.contains(".storage.modifier.")
        || scope_name.starts_with("storage.control")
        || scope_name.contains(".storage.control.")
}

fn is_variable_scope(scope_name: &str) -> bool {
    scope_name.starts_with("variable")
        || scope_name.contains(".variable.")
        || scope_name.starts_with("entity.name.variable")
        || scope_name.contains(".entity.name.variable.")
        || scope_name.starts_with("support.variable")
        || scope_name.contains(".support.variable.")
}

fn is_operator_scope(scope_name: &str) -> bool {
    scope_name.starts_with("keyword.operator")
        || scope_name.contains(".keyword.operator.")
        || scope_name.starts_with("punctuation")
        || scope_name.contains(".punctuation.")
}
