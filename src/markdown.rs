use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::theme::Theme;

pub struct StyledLine<'a> {
    pub line: Line<'a>,
    pub is_blank: bool,
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
    let mut in_heading: Option<u8> = None;
    let mut in_code_block = false;
    let mut in_blockquote = false;
    let mut in_list_item = false;
    let mut list_number: Option<u64> = None;
    let mut list_item_first_para = false;
    let mut list_indent: usize = 0;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    flush_line(&mut lines, &mut current_spans);
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
                    list_number = start;
                }
                Tag::Item => {
                    flush_line(&mut lines, &mut current_spans);
                    in_list_item = true;
                    list_item_first_para = true;
                }
                Tag::Emphasis => italic = true,
                Tag::Strong => bold = true,
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Heading(_) => {
                    flush_line(&mut lines, &mut current_spans);
                    push_blank(&mut lines);
                    in_heading = None;
                }
                TagEnd::Paragraph => {
                    flush_line(&mut lines, &mut current_spans);
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
                    list_number = None;
                    push_blank(&mut lines);
                }
                TagEnd::Item => {
                    flush_line(&mut lines, &mut current_spans);
                    in_list_item = false;
                }
                TagEnd::Emphasis => italic = false,
                TagEnd::Strong => bold = false,
                _ => {}
            },
            Event::Text(text) => {
                let text = text.into_string();
                let style = if let Some(level) = in_heading {
                    let color = match level {
                        1 => theme.accent,
                        2 => theme.heading,
                        _ => theme.text_bright,
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
                    s
                };

                if in_code_block {
                    for line_text in text.lines() {
                        current_spans.push(Span::styled(
                            "  ".to_string(),
                            Style::default().fg(theme.text_muted),
                        ));
                        current_spans.push(Span::styled(line_text.to_string(), style));
                        flush_line(&mut lines, &mut current_spans);
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
                        let bullet = if let Some(ref mut n) = list_number {
                            let s = format!("  {}. ", n);
                            *n += 1;
                            s
                        } else {
                            "  • ".to_string()
                        };
                        list_indent = bullet.len();
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
                        let wlen = word.len();
                        if line_len > 0 && line_len + 1 + wlen > max_width {
                            flush_line(&mut lines, &mut current_spans);
                            // Add continuation indent for list items
                            if in_list_item && list_indent > 0 {
                                let indent = " ".repeat(list_indent);
                                current_spans.push(Span::raw(indent.clone()));
                                line_len = indent.len();
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
                    let bullet = if let Some(ref mut n) = list_number {
                        let s = format!("  {}. ", n);
                        *n += 1;
                        s
                    } else {
                        "  • ".to_string()
                    };
                    list_indent = bullet.len();
                    current_spans.push(Span::styled(
                        bullet,
                        Style::default().fg(theme.accent),
                    ));
                    list_item_first_para = false;
                }

                current_spans.push(Span::styled(format!("`{}`", text), style));
            }
            Event::TaskListMarker(checked) => {
                let marker = if checked { "  [x] " } else { "  [ ] " };
                list_indent = marker.len();
                current_spans.push(Span::styled(
                    marker.to_string(),
                    Style::default().fg(theme.accent),
                ));
                list_item_first_para = false;
            }
            Event::SoftBreak | Event::HardBreak => {
                flush_line(&mut lines, &mut current_spans);
            }
            Event::Rule => {
                flush_line(&mut lines, &mut current_spans);
                let rule = "─".repeat(width.saturating_sub(2) as usize);
                lines.push(StyledLine {
                    line: Line::from(Span::styled(
                        rule,
                        Style::default().fg(theme.border),
                    )),
                    is_blank: false,
                });
                push_blank(&mut lines);
            }
            _ => {}
        }
    }

    flush_line(&mut lines, &mut current_spans);

    // Remove trailing blank lines
    while lines.last().map_or(false, |l| l.is_blank) {
        lines.pop();
    }

    lines
}

fn current_line_len(spans: &[Span]) -> usize {
    spans.iter().map(|s| s.content.len()).sum()
}

fn push_blank(lines: &mut Vec<StyledLine<'static>>) {
    lines.push(StyledLine {
        line: Line::default(),
        is_blank: true,
    });
}

fn flush_line(
    lines: &mut Vec<StyledLine<'static>>,
    spans: &mut Vec<Span<'static>>,
) {
    if spans.is_empty() {
        return;
    }
    let line = Line::from(std::mem::take(spans));
    lines.push(StyledLine {
        line,
        is_blank: false,
    });
}
