use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;

fn syntect_highlight(code: &str, lang: &str, _theme: &Theme) -> Vec<Vec<Span<'static>>> {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ss
        .find_syntax_by_token(lang)
        .unwrap_or_else(|| ss.find_syntax_plain_text());
    let syn_theme = &ts.themes["base16-ocean.dark"];

    let mut highlighter = syntect::easy::HighlightLines::new(syntax, syn_theme);
    let mut result = Vec::new();

    for line in syntect::util::LinesWithEndings::from(code) {
        let ranges = highlighter.highlight_line(line, &ss).unwrap_or_default();
        let spans: Vec<Span<'static>> = ranges
            .into_iter()
            .map(|(style, text)| {
                let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                Span::styled(
                    text.trim_end_matches('\n').to_string(),
                    Style::default().fg(fg),
                )
            })
            .collect();
        result.push(spans);
    }

    // If code doesn't end with newline, we still got all lines
    // If it's empty, return one empty line
    if result.is_empty() {
        result.push(Vec::new());
    }

    result
}

#[allow(dead_code)]
pub struct StyledLine<'a> {
    pub line: Line<'a>,
    pub is_blank: bool,
    pub is_heading: bool,
    pub heading_level: Option<u8>,
    pub heading_text: Option<String>,
    /// Source line number for task list items (used for checkbox toggling)
    pub source_line: Option<usize>,
    /// Inline tags found on this line (e.g. "feature", "bug")
    pub tags: Vec<String>,
    /// Link destination if this line contains a markdown link
    pub link_url: Option<String>,
    /// True if this line starts a new list item (bullet or task)
    pub is_list_item_start: bool,
}

/// Resolve a tag name to its label color, or a default tag color.
pub fn tag_color(tag: &str, theme: &Theme) -> Color {
    match tag {
        "bug" | "bugs" => theme.labels.bugs,
        "feature" | "features" => theme.labels.features,
        "improvement" | "improvements" => theme.labels.improvements,
        "refactor" | "refactoring" => theme.labels.refactor,
        "doc" | "docs" | "documentation" => theme.labels.docs,
        "chore" | "chores" => theme.labels.chore,
        "data" => theme.labels.data,
        "model" => theme.labels.model,
        "experiment" | "experiments" => theme.labels.experiment,
        _ => theme.text_dim,
    }
}

pub fn parse_markdown(source: &str, theme: Theme, width: u16) -> Vec<StyledLine<'static>> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    opts.insert(Options::ENABLE_WIKILINKS);

    let parser = Parser::new_ext(source, opts);
    let mut lines: Vec<StyledLine<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut current_tags: Vec<String> = Vec::new();
    let mut current_link_url: Option<String> = None;

    let mut bold = false;
    let mut italic = false;
    let mut strikethrough = false;
    let mut in_heading: Option<u8> = None;
    let mut heading_text_buf = String::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_buf = String::new();
    let mut in_blockquote = false;
    let mut in_list_item = false;
    // Stack of (ordered_start, next_number) for nested lists
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut list_item_first_para = false;
    let mut list_indent: usize = 0;
    let mut pending_list_item_start = false;
    let mut in_table = false;
    let mut table_cell_buf = String::new();
    let mut table_row: Vec<String> = Vec::new();
    let mut table_rows: Vec<Vec<String>> = Vec::new(); // all rows (header first)
    let mut table_has_header = false;
    let mut table_alignments: Vec<pulldown_cmark::Alignment> = Vec::new();

    // Track source line for task list items
    let mut task_source_line: Option<usize> = None;
    let mut in_metadata = false;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::MetadataBlock(_)) => { in_metadata = true; continue; }
            Event::End(TagEnd::MetadataBlock(_)) => { in_metadata = false; continue; }
            _ if in_metadata => continue,
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    flush_line(&mut lines, &mut current_spans, task_source_line, &mut current_tags, &mut current_link_url, &mut pending_list_item_start);
                    in_heading = Some(level as u8);
                }
                Tag::Paragraph => {}
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    code_block_buf.clear();
                    code_block_lang = match kind {
                        CodeBlockKind::Fenced(lang) => lang.split_whitespace().next().unwrap_or("").to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                }
                Tag::BlockQuote(_) => {
                    in_blockquote = true;
                }
                Tag::List(start) => {
                    list_stack.push(start);
                }
                Tag::Item => {
                    flush_line(&mut lines, &mut current_spans, task_source_line, &mut current_tags, &mut current_link_url, &mut pending_list_item_start);
                    in_list_item = true;
                    list_item_first_para = true;
                    pending_list_item_start = true;
                    task_source_line = None;
                    // Calculate indent based on nesting depth (depth >= 2 means nested)
                    let depth = list_stack.len();
                    list_indent = if depth > 1 { (depth - 1) * 4 + 2 } else { 2 };
                }
                Tag::Emphasis => italic = true,
                Tag::Strong => bold = true,
                Tag::Strikethrough => strikethrough = true,
                Tag::Table(alignments) => {
                    in_table = true;
                    table_alignments = alignments;
                }
                Tag::Link { link_type, dest_url, .. } => {
                    let url = dest_url.to_string();
                    current_link_url = Some(match link_type {
                        LinkType::WikiLink { .. } => format!("{}.md", url),
                        _ => url,
                    });
                }
                Tag::TableHead => {}
                Tag::TableRow => {}
                Tag::TableCell => {
                    table_cell_buf.clear();
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Heading(_) => {
                    let level = in_heading.unwrap_or(1);
                    let text = std::mem::take(&mut heading_text_buf);
                    flush_line_heading(&mut lines, &mut current_spans, level, text, &mut current_tags, &mut current_link_url);
                    push_blank(&mut lines);
                    in_heading = None;
                }
                TagEnd::Paragraph => {
                    flush_line(&mut lines, &mut current_spans, task_source_line, &mut current_tags, &mut current_link_url, &mut pending_list_item_start);
                    if !in_list_item {
                        push_blank(&mut lines);
                    }
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    // Emit highlighted code lines
                    let highlighted = syntect_highlight(&code_block_buf, &code_block_lang, &theme);
                    for spans in highlighted {
                        let mut line_spans = vec![Span::styled(
                            "  ".to_string(),
                            Style::default().fg(theme.text_muted),
                        )];
                        line_spans.extend(spans);
                        lines.push(StyledLine {
                            line: Line::from(line_spans),
                            is_blank: false,
                            is_heading: false,
                            heading_level: None,
                            heading_text: None,
                            source_line: None,
                            tags: Vec::new(),
                            link_url: None,
                            is_list_item_start: false,
                        });
                    }
                    push_blank(&mut lines);
                }
                TagEnd::BlockQuote(_) => {
                    in_blockquote = false;
                }
                TagEnd::List(_) => {
                    list_stack.pop();
                    if list_stack.is_empty() {
                        push_blank(&mut lines);
                    }
                }
                TagEnd::Item => {
                    flush_line(&mut lines, &mut current_spans, task_source_line, &mut current_tags, &mut current_link_url, &mut pending_list_item_start);
                    in_list_item = false;
                    task_source_line = None;
                }
                TagEnd::Emphasis => italic = false,
                TagEnd::Strong => bold = false,
                TagEnd::Strikethrough => strikethrough = false,
                TagEnd::Table => {
                    in_table = false;
                    // Compute column widths from all accumulated rows
                    let num_cols = table_alignments.len();
                    let mut col_widths: Vec<usize> = vec![0; num_cols];
                    for row in &table_rows {
                        for (j, cell) in row.iter().enumerate() {
                            if j < num_cols {
                                col_widths[j] = col_widths[j].max(cell.len());
                            }
                        }
                    }
                    // Emit all rows
                    for (row_idx, row) in table_rows.iter().enumerate() {
                        emit_table_row(&mut lines, row, &table_alignments, &col_widths, theme, row_idx == 0 && table_has_header);
                        // Separator after header
                        if row_idx == 0 && table_has_header {
                            let sep: String = col_widths.iter()
                                .map(|&w| "─".repeat(w))
                                .collect::<Vec<_>>()
                                .join("─┼─");
                            lines.push(StyledLine {
                                line: Line::from(Span::styled(
                                    format!("  {}", sep),
                                    Style::default().fg(theme.border),
                                )),
                                is_blank: false,
                                is_heading: false,
                                heading_level: None,
                                heading_text: None,
                                source_line: None,
                                tags: Vec::new(),
                                link_url: None,
                                is_list_item_start: false,
                            });
                        }
                    }
                    table_rows.clear();
                    table_has_header = false;
                    table_alignments.clear();
                    push_blank(&mut lines);
                }
                TagEnd::TableHead => {
                    table_rows.push(std::mem::take(&mut table_row));
                    table_has_header = true;
                }
                TagEnd::TableRow => {
                    table_rows.push(std::mem::take(&mut table_row));
                }
                TagEnd::TableCell => {
                    table_row.push(std::mem::take(&mut table_cell_buf));
                }
                _ => {}
            },
            Event::Text(text) => {
                let text = text.into_string();
                if in_heading.is_some() {
                    heading_text_buf.push_str(&text);
                }

                if in_code_block {
                    code_block_buf.push_str(&text);
                    continue;
                }

                if in_table {
                    table_cell_buf.push_str(&text);
                    continue;
                }

                let style = if let Some(level) = in_heading {
                    let color = match level {
                        1 => theme.accent,
                        2 => theme.heading,
                        _ => theme.text_bright,
                    };
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                } else {
                    let mut s = Style::default().fg(theme.text);
                    if bold {
                        s = s.add_modifier(Modifier::BOLD);
                    }
                    if italic {
                        s = s.add_modifier(Modifier::ITALIC);
                    }
                    if strikethrough {
                        s = s.add_modifier(Modifier::CROSSED_OUT);
                    }
                    s
                };

                if in_blockquote {
                    current_spans.push(Span::styled(
                        "  │ ".to_string(),
                        Style::default().fg(theme.border),
                    ));
                    current_spans.push(Span::styled(text, style));
                } else {
                    if let Some(level) = in_heading {
                        let prefix = match level {
                            1 => "# ",
                            2 => "## ",
                            3 => "### ",
                            _ => "#### ",
                        };
                        current_spans.push(Span::styled(prefix.to_string(), style));
                    }

                    // Emit bullet/number prefix for first text in a list item
                    if list_item_first_para {
                        let depth = list_stack.len();
                        let indent = if depth > 1 { " ".repeat((depth - 1) * 4) } else { String::new() };
                        let bullet = match list_stack.last_mut() {
                            Some(Some(n)) => {
                                let s = format!("  {}{}. ", indent, n);
                                *n += 1;
                                s
                            }
                            _ => format!("  {}• ", indent),
                        };
                        list_indent = bullet.width();
                        current_spans.push(Span::styled(
                            bullet,
                            Style::default().fg(theme.accent),
                        ));
                        list_item_first_para = false;
                    }

                    // Word wrapping with inline #tag detection
                    let max_width = width.saturating_sub(2) as usize;
                    let words: Vec<&str> = text.split_whitespace().collect();
                    let mut line_len = current_line_len(&current_spans);

                    for word in words {
                        let wlen = word.width();
                        if line_len > 0 && line_len + 1 + wlen > max_width {
                            flush_line(&mut lines, &mut current_spans, task_source_line, &mut current_tags, &mut current_link_url, &mut pending_list_item_start);
                            // Add continuation indent for list items
                            if in_list_item && list_indent > 0 {
                                let indent = " ".repeat(list_indent);
                                current_spans.push(Span::raw(indent.clone()));
                                line_len = list_indent;
                            } else {
                                line_len = 0;
                            }
                        }
                        if line_len > 0 {
                            current_spans.push(Span::raw(" ".to_string()));
                            line_len += 1;
                        }

                        // Detect #tags (not inside headings or code blocks)
                        if in_heading.is_none() && !in_code_block {
                            if let Some(tag_name) = word.strip_prefix('#') {
                                if !tag_name.is_empty() && tag_name.chars().any(|c| c.is_alphabetic()) && tag_name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                                    let color = tag_color(&tag_name.to_lowercase(), &theme);
                                    current_spans.push(Span::styled(
                                        word.to_string(),
                                        Style::default().fg(color),
                                    ));
                                    current_tags.push(tag_name.to_lowercase());
                                    line_len += wlen;
                                    continue;
                                }
                            }
                        }

                        current_spans.push(Span::styled(word.to_string(), style));
                        line_len += wlen;
                    }
                }
            }
            Event::Code(text) => {
                let text = text.into_string();

                if in_table {
                    table_cell_buf.push_str(&text);
                    continue;
                }

                // Check if we need to emit bullet prefix first
                if list_item_first_para {
                    let depth = list_stack.len();
                    let indent = if depth > 1 { " ".repeat((depth - 1) * 4) } else { String::new() };
                    let bullet = match list_stack.last_mut() {
                        Some(Some(n)) => {
                            let s = format!("  {}{}. ", indent, n);
                            *n += 1;
                            s
                        }
                        _ => format!("  {}• ", indent),
                    };
                    list_indent = bullet.width();
                    current_spans.push(Span::styled(
                        bullet,
                        Style::default().fg(theme.accent),
                    ));
                    list_item_first_para = false;
                }

                if current_line_len(&current_spans) > 0 {
                    current_spans.push(Span::raw(" ".to_string()));
                }
                let code_style = Style::default().fg(theme.accent).bg(theme.cursor_bg);
                current_spans.push(Span::styled(format!(" {} ", text), code_style));
            }
            Event::TaskListMarker(checked) => {
                // Compute source line number from byte offset
                task_source_line = Some(source[..range.start].bytes().filter(|&b| b == b'\n').count());
                let marker = if checked { "  [x] " } else { "  [ ] " };
                list_indent = marker.width();
                current_spans.push(Span::styled(
                    marker.to_string(),
                    Style::default().fg(theme.accent),
                ));
                list_item_first_para = false;
            }
            Event::SoftBreak | Event::HardBreak => {
                flush_line(&mut lines, &mut current_spans, task_source_line, &mut current_tags, &mut current_link_url, &mut pending_list_item_start);
            }
            Event::Rule => {
                flush_line(&mut lines, &mut current_spans, task_source_line, &mut current_tags, &mut current_link_url, &mut pending_list_item_start);
                let rule = "─".repeat(width.saturating_sub(2) as usize);
                lines.push(StyledLine {
                    line: Line::from(Span::styled(
                        rule,
                        Style::default().fg(theme.border),
                    )),
                    is_blank: false,
                    is_heading: false,
                    heading_level: None,
                    heading_text: None,
                    source_line: None,
                    tags: Vec::new(),
                    link_url: None,
                    is_list_item_start: false,
                });
                push_blank(&mut lines);
            }
            _ => {}
        }
    }

    flush_line(&mut lines, &mut current_spans, task_source_line, &mut current_tags, &mut current_link_url, &mut pending_list_item_start);

    // Remove trailing blank lines
    while lines.last().map_or(false, |l| l.is_blank) {
        lines.pop();
    }

    lines
}

fn emit_table_row(
    lines: &mut Vec<StyledLine<'static>>,
    cells: &[String],
    alignments: &[pulldown_cmark::Alignment],
    col_widths: &[usize],
    theme: Theme,
    is_header: bool,
) {
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::styled("  ".to_string(), Style::default()));

    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(theme.border)));
        }

        let col_width = col_widths.get(i).copied().unwrap_or(cell.len());
        let alignment = alignments.get(i).copied().unwrap_or(pulldown_cmark::Alignment::None);
        let text = match alignment {
            pulldown_cmark::Alignment::Right => format!("{:>width$}", cell, width = col_width),
            pulldown_cmark::Alignment::Center => format!("{:^width$}", cell, width = col_width),
            _ => format!("{:<width$}", cell, width = col_width),
        };

        let style = if is_header {
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };
        spans.push(Span::styled(text, style));
    }

    lines.push(StyledLine {
        line: Line::from(spans),
        is_blank: false,
        is_heading: false,
        heading_level: None,
        heading_text: None,
        source_line: None,
        tags: Vec::new(),
        link_url: None,
        is_list_item_start: false,
    });
}

fn current_line_len(spans: &[Span]) -> usize {
    spans.iter().map(|s| s.content.width()).sum()
}

fn push_blank(lines: &mut Vec<StyledLine<'static>>) {
    lines.push(StyledLine {
        line: Line::default(),
        is_blank: true,
        is_heading: false,
        heading_level: None,
        heading_text: None,
        source_line: None,
        tags: Vec::new(),
        link_url: None,
        is_list_item_start: false,
    });
}

fn flush_line(
    lines: &mut Vec<StyledLine<'static>>,
    spans: &mut Vec<Span<'static>>,
    source_line: Option<usize>,
    tags: &mut Vec<String>,
    link_url: &mut Option<String>,
    is_list_item_start: &mut bool,
) {
    if spans.is_empty() {
        return;
    }
    let line = Line::from(std::mem::take(spans));
    let starts_item = std::mem::replace(is_list_item_start, false);
    lines.push(StyledLine {
        line,
        is_blank: false,
        is_heading: false,
        heading_level: None,
        heading_text: None,
        source_line,
        tags: std::mem::take(tags),
        link_url: link_url.take(),
        is_list_item_start: starts_item,
    });
}

fn flush_line_heading(
    lines: &mut Vec<StyledLine<'static>>,
    spans: &mut Vec<Span<'static>>,
    level: u8,
    text: String,
    tags: &mut Vec<String>,
    link_url: &mut Option<String>,
) {
    if spans.is_empty() {
        return;
    }
    let line = Line::from(std::mem::take(spans));
    lines.push(StyledLine {
        line,
        is_blank: false,
        is_heading: true,
        heading_level: Some(level),
        heading_text: Some(text),
        source_line: None,
        tags: std::mem::take(tags),
        link_url: link_url.take(),
        is_list_item_start: false,
    });
}
