fn ai_workspace_wrap_text(text: &str, max_chars_per_line: usize, max_lines: usize) -> Vec<String> {
    ai_workspace_wrap_text_ranges(text, max_chars_per_line, max_lines)
        .into_iter()
        .map(|range| text[range].to_string())
        .collect()
}

fn ai_workspace_wrap_text_ranges(
    text: &str,
    max_chars_per_line: usize,
    max_lines: usize,
) -> Vec<Range<usize>> {
    if max_lines == 0 {
        return Vec::new();
    }

    let max_chars_per_line = max_chars_per_line.max(8);
    let mut lines = Vec::new();

    let mut raw_lines = text.lines().peekable();
    let mut cursor = 0usize;
    while let Some(raw_line) = raw_lines.next() {
        let has_more_input = raw_lines.peek().is_some();
        let raw_line_start = cursor;
        let raw_line_end = raw_line_start + raw_line.len();
        cursor = raw_line_end.saturating_add(1);
        if raw_line.is_empty() {
            lines.push(raw_line_start..raw_line_start);
            if lines.len() == max_lines {
                if has_more_input {
                    ai_workspace_append_ellipsis_range(lines.last_mut(), text);
                }
                return lines;
            }
            continue;
        }

        let mut remaining_start = raw_line_start;
        let mut remaining = raw_line.trim_end_matches(['\r', ' ']);
        loop {
            if remaining.is_empty() {
                break;
            }

            let remaining_chars = remaining.chars().count();
            if remaining_chars <= max_chars_per_line {
                let trimmed_len = remaining.len();
                lines.push(remaining_start..remaining_start.saturating_add(trimmed_len));
                if lines.len() == max_lines {
                    if has_more_input {
                        ai_workspace_append_ellipsis_range(lines.last_mut(), text);
                    }
                    return lines;
                }
                break;
            }

            let split_index = ai_workspace_wrap_split_index(remaining, max_chars_per_line)
                .unwrap_or(remaining.len());
            let (chunk, rest) = remaining.split_at(split_index);
            let chunk = chunk.trim_end_matches([' ', '\t']);
            lines.push(if chunk.is_empty() {
                remaining_start..remaining_start.saturating_add(split_index)
            } else {
                remaining_start..remaining_start.saturating_add(chunk.len())
            });
            if lines.len() == max_lines {
                ai_workspace_append_ellipsis_range(lines.last_mut(), text);
                return lines;
            }
            remaining_start = remaining_start.saturating_add(split_index).saturating_add(
                rest.len()
                    .saturating_sub(rest.trim_start_matches([' ', '\t']).len()),
            );
            remaining = rest.trim_start_matches([' ', '\t']);
        }
    }

    lines
}

fn ai_workspace_wrap_split_index(text: &str, max_chars_per_line: usize) -> Option<usize> {
    let mut char_count = 0usize;
    let mut last_whitespace_break = None;

    for (byte_index, ch) in text.char_indices() {
        char_count = char_count.saturating_add(1);
        if ch.is_whitespace() {
            last_whitespace_break = Some(byte_index + ch.len_utf8());
        }
        if char_count >= max_chars_per_line {
            return last_whitespace_break.or(Some(byte_index + ch.len_utf8()));
        }
    }

    None
}

fn ai_workspace_append_ellipsis(line: Option<&mut String>) {
    let Some(line) = line else {
        return;
    };
    if !line.ends_with("...") {
        line.push_str("...");
    }
}

fn ai_workspace_append_ellipsis_range(line: Option<&mut Range<usize>>, text: &str) {
    let Some(line) = line else {
        return;
    };
    let mut end = line.end;
    while end > line.start && !text.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    line.end = end;
}

fn ai_workspace_offset_link_ranges(
    link_ranges: Vec<MarkdownLinkRange>,
    offset: usize,
) -> Vec<MarkdownLinkRange> {
    link_ranges
        .into_iter()
        .map(|range| MarkdownLinkRange {
            range: (range.range.start + offset)..(range.range.end + offset),
            raw_target: range.raw_target,
        })
        .collect()
}

fn ai_workspace_offset_style_spans(
    style_spans: Vec<AiWorkspacePreviewStyleSpan>,
    offset: usize,
) -> Vec<AiWorkspacePreviewStyleSpan> {
    style_spans
        .into_iter()
        .map(|span| AiWorkspacePreviewStyleSpan {
            range: (span.range.start + offset)..(span.range.end + offset),
            ..span
        })
        .collect()
}

fn ai_workspace_markdown_inline_text_and_styles(
    spans: &[hunk_domain::markdown_preview::MarkdownInlineSpan],
) -> (
    String,
    Vec<MarkdownLinkRange>,
    Vec<AiWorkspacePreviewStyleSpan>,
) {
    let (text, link_ranges) = markdown_inline_text_and_link_ranges(spans);
    let mut style_spans = Vec::new();
    let mut cursor = 0usize;

    for span in spans {
        if span.style.hard_break {
            if !text[..cursor].ends_with('\n') {
                cursor += 1;
            }
            continue;
        }
        if span.text.is_empty() {
            continue;
        }

        let start = cursor;
        let end = start + span.text.len();
        cursor = end;
        if !(span.style.bold
            || span.style.italic
            || span.style.strikethrough
            || span.style.code
            || span.style.link.is_some())
        {
            continue;
        }

        style_spans.push(AiWorkspacePreviewStyleSpan {
            range: start..end,
            color_role: None,
            bold: span.style.bold,
            italic: span.style.italic,
            strikethrough: span.style.strikethrough,
            code: span.style.code,
            link: span.style.link.is_some(),
        });
    }

    (text, link_ranges, style_spans)
}

fn ai_workspace_markdown_code_line_text_and_spans(
    spans: &[hunk_domain::markdown_preview::MarkdownCodeSpan],
) -> (String, Vec<AiWorkspacePreviewSyntaxSpan>) {
    let mut text = String::new();
    let mut syntax_spans = Vec::new();
    let mut cursor = 0usize;

    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        let start = cursor;
        text.push_str(span.text.as_str());
        cursor += span.text.len();
        syntax_spans.push(AiWorkspacePreviewSyntaxSpan {
            range: start..cursor,
            token: span.token,
        });
    }

    (text, syntax_spans)
}

fn ai_workspace_diff_summary_line_style_spans(
    line: &str,
    text_width_px: usize,
) -> Vec<AiWorkspacePreviewStyleSpan> {
    let max_chars_per_line = ai_workspace_chars_per_line(text_width_px, false, false);
    let visible_range = ai_workspace_wrap_text_ranges(line, max_chars_per_line, 1)
        .into_iter()
        .next()
        .unwrap_or(0..line.len());
    let visible_line = &line[visible_range];

    let mut spans = Vec::new();
    if let Some(path_start) = visible_line.find("Edited ") {
        let path_start = path_start + "Edited ".len();
        let path_end = visible_line[path_start..]
            .find("  +")
            .map(|offset| path_start + offset)
            .unwrap_or(visible_line.len());
        if path_start < path_end {
            spans.push(AiWorkspacePreviewStyleSpan {
                range: path_start..path_end,
                color_role: Some(AiWorkspacePreviewColorRole::Accent),
                bold: true,
                italic: false,
                strikethrough: false,
                code: false,
                link: false,
            });
        }
    }
    spans.extend(ai_workspace_diff_stat_style_spans(visible_line));
    spans
}

fn ai_workspace_diff_stat_style_spans(line: &str) -> Vec<AiWorkspacePreviewStyleSpan> {
    let mut spans = Vec::new();
    let mut cursor = 0usize;
    while cursor < line.len() {
        let Some(offset) = line[cursor..].find(['+', '-']) else {
            break;
        };
        let start = cursor + offset;
        let sign = line.as_bytes()[start] as char;
        let mut end = start + 1;
        while end < line.len() && line.as_bytes()[end].is_ascii_digit() {
            end += 1;
        }
        if end > start + 1 {
            spans.push(AiWorkspacePreviewStyleSpan {
                range: start..end,
                color_role: Some(if sign == '+' {
                    AiWorkspacePreviewColorRole::Added
                } else {
                    AiWorkspacePreviewColorRole::Removed
                }),
                bold: false,
                italic: false,
                strikethrough: false,
                code: false,
                link: false,
            });
        }
        cursor = end;
    }
    spans
}

fn ai_workspace_clip_copy_regions(
    copy_regions: &[AiWorkspaceCopyRegion],
    wrapped_starts: &[usize],
    wrapped_ends: &[usize],
    wrapped_line_count: usize,
) -> Vec<AiWorkspaceCopyRegion> {
    copy_regions
        .iter()
        .filter_map(|region| {
            let start = wrapped_starts.get(region.line_range.start).copied()?;
            let end = region
                .line_range
                .end
                .checked_sub(1)
                .and_then(|last| wrapped_ends.get(last).copied())
                .unwrap_or(start);
            let end = end.min(wrapped_line_count);
            (start < end).then(|| AiWorkspaceCopyRegion {
                line_range: start..end,
                text: region.text.clone(),
                tooltip: region.tooltip,
                success_message: region.success_message,
            })
        })
        .collect()
}

fn ai_workspace_clip_link_ranges(
    link_ranges: &[MarkdownLinkRange],
    visible_range: Range<usize>,
) -> Vec<MarkdownLinkRange> {
    link_ranges
        .iter()
        .filter_map(|range| {
            let start = range.range.start.max(visible_range.start);
            let end = range.range.end.min(visible_range.end);
            (start < end).then(|| MarkdownLinkRange {
                range: (start - visible_range.start)..(end - visible_range.start),
                raw_target: range.raw_target.clone(),
            })
        })
        .collect()
}

fn ai_workspace_clip_style_spans(
    style_spans: &[AiWorkspacePreviewStyleSpan],
    visible_range: Range<usize>,
) -> Vec<AiWorkspacePreviewStyleSpan> {
    style_spans
        .iter()
        .filter_map(|span| {
            let start = span.range.start.max(visible_range.start);
            let end = span.range.end.min(visible_range.end);
            (start < end).then(|| AiWorkspacePreviewStyleSpan {
                range: (start - visible_range.start)..(end - visible_range.start),
                color_role: span.color_role,
                bold: span.bold,
                italic: span.italic,
                strikethrough: span.strikethrough,
                code: span.code,
                link: span.link,
            })
        })
        .collect()
}

fn ai_workspace_clip_syntax_spans(
    syntax_spans: &[AiWorkspacePreviewSyntaxSpan],
    visible_range: Range<usize>,
) -> Vec<AiWorkspacePreviewSyntaxSpan> {
    syntax_spans
        .iter()
        .filter_map(|span| {
            let start = span.range.start.max(visible_range.start);
            let end = span.range.end.min(visible_range.end);
            (start < end).then(|| AiWorkspacePreviewSyntaxSpan {
                range: (start - visible_range.start)..(end - visible_range.start),
                token: span.token,
            })
        })
        .collect()
}
