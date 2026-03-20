use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::theme::Theme;

pub struct StyledLine<'a> {
    pub line: Line<'a>,
    pub is_blank: bool,
    pub is_heading: bool,
    pub heading_level: Option<u8>,
    pub heading_text: Option<String>,
    /// Source line number for task list items (used for checkbox toggling)
    pub source_line: Option<usize>,
}

pub fn parse_markdown(source: &str, theme: Theme, width: u16) -> Vec<StyledLine<'static>> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(source, opts);
    let mut lines: Vec<StyledLine<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();

    let mut bold = false;
    let mut italic = false;
    let mut strikethrough = false;
    let mut in_heading: Option<u8> = None;
    let mut heading_text_buf = String::new();
    let mut in_code_block = false;
    let mut in_blockquote = false;
    let mut in_list_item = false;
    // Stack of (ordered_start, next_number) for nested lists
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut list_item_first_para = false;
    let mut list_indent: usize = 0;
    let mut in_table = false;
    let mut table_row: Vec<String> = Vec::new();
    let mut table_alignments: Vec<pulldown_cmark::Alignment> = Vec::new();

    // Track source line for task list items
    let mut task_source_line: Option<usize> = None;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    flush_line(&mut lines, &mut current_spans, task_source_line);
                    in_heading = Some(level as u8);
                }
                Tag::Paragraph => {}
                Tag::CodeBlock(_) => {
                    in_code_block = true;
                }
                Tag::BlockQuote(_) => {
                    in_blockquote = true;
                }
                Tag::List(start) => {
                    list_stack.push(start);
                }
                Tag::Item => {
                    flush_line(&mut lines, &mut current_spans, task_source_line);
                    in_list_item = true;
                    list_item_first_para = true;
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
                Tag::TableHead => {}
                Tag::TableRow => {}
                Tag::TableCell => {}
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Heading(_) => {
                    let level = in_heading.unwrap_or(1);
                    let text = std::mem::take(&mut heading_text_buf);
                    flush_line_heading(&mut lines, &mut current_spans, level, text);
                    push_blank(&mut lines);
                    in_heading = None;
                }
                TagEnd::Paragraph => {
                    flush_line(&mut lines, &mut current_spans, task_source_line);
                    if !in_list_item {
                        push_blank(&mut lines);
                    }
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
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
                    flush_line(&mut lines, &mut current_spans, task_source_line);
                    in_list_item = false;
                    task_source_line = None;
                }
                TagEnd::Emphasis => italic = false,
                TagEnd::Strong => bold = false,
                TagEnd::Strikethrough => strikethrough = false,
                TagEnd::Table => {
                    in_table = false;
                    table_alignments.clear();
                    push_blank(&mut lines);
                }
                TagEnd::TableHead => {
                    // Emit header row
                    emit_table_row(&mut lines, &table_row, &table_alignments, theme, true, width);
                    table_row.clear();
                    // Emit separator
                    let sep = table_alignments
                        .iter()
                        .map(|_| "───────")
                        .collect::<Vec<_>>()
                        .join("─┼─");
                    lines.push(StyledLine {
                        line: Line::from(Span::styled(
                            format!("  {}",sep),
                            Style::default().fg(theme.border),
                        )),
                        is_blank: false,
                        is_heading: false,
                        heading_level: None,
                        heading_text: None,
                        source_line: None,
                    });
                }
                TagEnd::TableRow => {
                    emit_table_row(&mut lines, &table_row, &table_alignments, theme, false, width);
                    table_row.clear();
                }
                TagEnd::TableCell => {}
                _ => {}
            },
            Event::Text(text) => {
                let text = text.into_string();
                if in_heading.is_some() {
                    heading_text_buf.push_str(&text);
                }

                if in_table {
                    table_row.push(text);
                    continue;
                }

                let style = if let Some(level) = in_heading {
                    let color = if level >= 3 {
                        label_color(&heading_text_buf, &theme).unwrap_or(theme.text_bright)
                    } else {
                        match level {
                            1 => theme.accent,
                            _ => theme.heading,
                        }
                    };
                    Style::default().fg(color).add_modifier(Modifier::BOLD)
                } else if in_code_block {
                    Style::default().fg(theme.text_dim)
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

                if in_code_block {
                    for line_text in text.lines() {
                        current_spans.push(Span::styled(
                            "  ".to_string(),
                            Style::default().fg(theme.text_muted),
                        ));
                        current_spans.push(Span::styled(line_text.to_string(), style));
                        flush_line(&mut lines, &mut current_spans, None);
                    }
                } else if in_blockquote {
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

                    // Word wrapping
                    let max_width = width.saturating_sub(2) as usize;
                    let words: Vec<&str> = text.split_whitespace().collect();
                    let mut line_len = current_line_len(&current_spans);

                    for word in words {
                        let wlen = word.width();
                        if line_len > 0 && line_len + 1 + wlen > max_width {
                            flush_line(&mut lines, &mut current_spans, task_source_line);
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
                        current_spans.push(Span::styled(word.to_string(), style));
                        line_len += wlen;
                    }
                }
            }
            Event::Code(text) => {
                let text = text.into_string();
                let style = Style::default().fg(theme.accent);

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

                current_spans.push(Span::styled(format!("`{}`", text), style));
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
                flush_line(&mut lines, &mut current_spans, task_source_line);
            }
            Event::Rule => {
                flush_line(&mut lines, &mut current_spans, task_source_line);
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
                });
                push_blank(&mut lines);
            }
            _ => {}
        }
    }

    flush_line(&mut lines, &mut current_spans, task_source_line);

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
    theme: Theme,
    is_header: bool,
    _width: u16,
) {
    let col_width = 15;
    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.push(Span::styled("  ".to_string(), Style::default()));

    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", Style::default().fg(theme.border)));
        }

        let alignment = alignments.get(i).copied().unwrap_or(pulldown_cmark::Alignment::None);
        let text = if cell.len() > col_width {
            format!("{}…", &cell[..col_width - 1])
        } else {
            match alignment {
                pulldown_cmark::Alignment::Right => format!("{:>width$}", cell, width = col_width),
                pulldown_cmark::Alignment::Center => format!("{:^width$}", cell, width = col_width),
                _ => format!("{:<width$}", cell, width = col_width),
            }
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
    });
}

fn flush_line(
    lines: &mut Vec<StyledLine<'static>>,
    spans: &mut Vec<Span<'static>>,
    source_line: Option<usize>,
) {
    if spans.is_empty() {
        return;
    }
    let line = Line::from(std::mem::take(spans));
    lines.push(StyledLine {
        line,
        is_blank: false,
        is_heading: false,
        heading_level: None,
        heading_text: None,
        source_line,
    });
}

fn label_color(heading_text: &str, theme: &Theme) -> Option<ratatui::style::Color> {
    let text = heading_text.trim().to_lowercase();
    match text.as_str() {
        "bugs" | "bug" => Some(theme.labels.bugs),
        "features" | "feature" => Some(theme.labels.features),
        "improvements" | "improvement" => Some(theme.labels.improvements),
        "refactor" | "refactoring" => Some(theme.labels.refactor),
        "docs" | "documentation" => Some(theme.labels.docs),
        "chore" | "chores" => Some(theme.labels.chore),
        _ => None,
    }
}

fn flush_line_heading(
    lines: &mut Vec<StyledLine<'static>>,
    spans: &mut Vec<Span<'static>>,
    level: u8,
    text: String,
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
    });
}
