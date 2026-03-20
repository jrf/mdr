use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::state::{AppMode, AppState};

fn shorten_path(path: &str) -> String {
    if let Some(home) = std::env::var_os("HOME") {
        let home = home.to_string_lossy();
        if let Some(rest) = path.strip_prefix(home.as_ref()) {
            return format!("~{}", rest);
        }
    }
    path.to_string()
}

pub fn draw(f: &mut Frame, state: &mut AppState) {
    draw_reader(f, state);
    match state.mode {
        AppMode::Reader | AppMode::Search => {}
        AppMode::FilePicker => draw_file_picker(f, state),
        AppMode::ThemePicker { .. } => draw_theme_picker(f, state),
        AppMode::Help => draw_help(f, state),
    }
}

fn render_entry_list(
    entries: &[(&crate::browser::BrowserEntry, bool)],
    theme: crate::theme::Theme,
    empty_msg: &str,
    scroll: usize,
    visible_height: usize,
    area: Rect,
    f: &mut Frame,
) {
    let lines: Vec<Line> = entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(_, (entry, is_selected))| {
            let prefix = "   ";

            let icon = if entry.name == ".." {
                "^ "
            } else if entry.is_dir {
                "/ "
            } else {
                "  "
            };

            let style = if *is_selected {
                Style::default()
                    .fg(theme.text_bright)
                    .bg(theme.cursor_bg)
                    .add_modifier(Modifier::BOLD)
            } else if entry.name == ".." {
                Style::default().fg(theme.text_dim)
            } else if entry.is_dir {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.text)
            };

            let mut line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(icon, style),
                Span::styled(entry.name.as_str(), style),
            ]);
            if *is_selected {
                let content_width: usize = line.spans.iter().map(|s| s.content.width()).sum();
                let area_width = area.width as usize;
                if content_width < area_width {
                    line.spans.push(Span::styled(
                        " ".repeat(area_width - content_width),
                        Style::default().bg(theme.cursor_bg),
                    ));
                }
            }
            line
        })
        .collect();

    if lines.is_empty() {
        let empty = Line::from(Span::styled(empty_msg, Style::default().fg(theme.text_dim)));
        f.render_widget(Paragraph::new(vec![empty]), area);
    } else {
        f.render_widget(Paragraph::new(lines), area);
    }
}

fn draw_tab_bar(f: &mut Frame, state: &AppState, area: Rect) {
    let theme = state.theme;
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(" ", Style::default()));

    for (i, tab) in state.tabs.iter().enumerate() {
        let name = tab
            .file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled".to_string());

        if i == state.active_tab {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default()
                    .fg(theme.text_bright)
                    .bg(theme.cursor_bg)
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default().fg(theme.text_dim),
            ));
        }
        spans.push(Span::styled(" ", Style::default()));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_reader(f: &mut Frame, state: &mut AppState) {
    let area = f.area();
    let theme = state.theme;

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .padding(Padding::horizontal(1));

    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    let show_tabs = state.tabs.len() > 1;

    let chunks = if show_tabs {
        Layout::vertical([
            Constraint::Length(1), // tab bar
            Constraint::Length(1), // spacer
            Constraint::Min(1),   // content
            Constraint::Length(1), // status bar
        ])
        .split(inner)
    } else {
        Layout::vertical([
            Constraint::Min(1),   // content
            Constraint::Length(1), // status bar
        ])
        .split(inner)
    };

    // Tab bar
    if show_tabs {
        draw_tab_bar(f, state, chunks[0]);
    }

    // Content area
    let content_area = if show_tabs { chunks[2] } else { chunks[0] };
    let status_area = if show_tabs { chunks[3] } else { chunks[1] };
    {
        let tab = state.tab_mut();
        let _parsed = tab.get_parsed_lines(content_area.width, theme);
        let display_indices = tab.visible_line_indices();
        let total_lines = display_indices.len();
        let visible_height = content_area.height as usize;
        tab.total_lines = total_lines;
        tab.visible_height = visible_height;
    }

    let tab = state.tab();
    let display_indices = tab.visible_line_indices();
    let total_lines = display_indices.len();
    let visible_height = content_area.height as usize;
    let scroll = tab.scroll.min(total_lines.saturating_sub(visible_height));
    let cursor = tab.cursor;
    let visible: Vec<Line> = display_indices[scroll..]
        .iter()
        .enumerate()
        .take(visible_height)
        .map(|(i, &line_idx)| {
            let sl = &tab.cached_lines[line_idx];
            let mut line = if tab.search_query.is_empty() {
                sl.line.clone()
            } else {
                highlight_search(sl.line.clone(), &tab.search_query, theme)
            };
            // Add fold indicator for headings
            if sl.is_heading {
                if let Some(ref text) = sl.heading_text {
                    let indicator = if tab.folded_headings.contains(text) {
                        "▶ "
                    } else {
                        "▼ "
                    };
                    line.spans.insert(0, Span::styled(
                        indicator.to_string(),
                        Style::default().fg(theme.text_dim),
                    ));
                }
            }
            // Highlight cursor line with a subtle background
            if scroll + i == cursor {
                let cursor_style = Style::default().bg(theme.cursor_bg);
                for span in &mut line.spans {
                    span.style = span.style.bg(theme.cursor_bg);
                }
                // Pad to full width so the highlight spans the line
                let content_width: usize = line.spans.iter().map(|s| s.content.width()).sum();
                let area_width = content_area.width as usize;
                if content_width < area_width {
                    line.spans.push(Span::styled(
                        " ".repeat(area_width - content_width),
                        cursor_style,
                    ));
                }
            }
            line
        })
        .collect();

    f.render_widget(Paragraph::new(visible), content_area);

    // Scrollbar
    if state.scrollbar && total_lines > visible_height {
        let mut scrollbar_state = ScrollbarState::new(total_lines.saturating_sub(visible_height))
            .position(scroll);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::default().fg(theme.text_dim))
                .track_style(Style::default().fg(theme.border)),
            content_area,
            &mut scrollbar_state,
        );
    }

    // Status bar
    let tab = state.tab();
    let filename = tab
        .file_path
        .as_ref()
        .map(|p| shorten_path(&p.display().to_string()))
        .unwrap_or_else(|| "no file".to_string());

    let is_searching = matches!(state.mode, AppMode::Search);

    if is_searching {
        let match_info = if tab.search_matches.is_empty() {
            if tab.search_query.is_empty() {
                String::new()
            } else {
                " (no matches)".to_string()
            }
        } else {
            format!(" ({}/{})", tab.search_current + 1, tab.search_matches.len())
        };

        let status = Line::from(vec![
            Span::styled("/", Style::default().fg(theme.accent)),
            Span::styled(
                tab.search_query.clone(),
                Style::default().fg(theme.text_bright),
            ),
            Span::styled(match_info, Style::default().fg(theme.text_dim)),
        ]);
        f.render_widget(Paragraph::new(status), status_area);
    } else {
        let scroll_pct = if total_lines <= visible_height {
            100
        } else {
            ((scroll as f64 / (total_lines - visible_height) as f64) * 100.0) as usize
        };

        let mut status_spans = vec![
            Span::styled(
                "mdr",
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            Span::styled(filename, Style::default().fg(theme.text)),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            Span::styled(
                format!("{}%", scroll_pct),
                Style::default().fg(theme.text_dim),
            ),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            Span::styled(
                "?:help",
                Style::default().fg(theme.text_muted),
            ),
        ];

        if tab.filter_tasks {
            status_spans.push(Span::styled(" │ ", Style::default().fg(theme.border)));
            status_spans.push(Span::styled(
                "[filter]",
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ));
        }

        if tab.file_updated {
            status_spans.push(Span::styled(" │ ", Style::default().fg(theme.border)));
            status_spans.push(Span::styled(
                "[updated]",
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ));
        }

        if !tab.search_query.is_empty() {
            status_spans.push(Span::styled(" │ ", Style::default().fg(theme.border)));
            let match_info = if tab.search_matches.is_empty() {
                format!("/{}", tab.search_query)
            } else {
                format!("/{} ({}/{})", tab.search_query, tab.search_current + 1, tab.search_matches.len())
            };
            status_spans.push(Span::styled(match_info, Style::default().fg(theme.text_dim)));
        }

        f.render_widget(Paragraph::new(Line::from(status_spans)), status_area);
    }
}

fn highlight_search<'a>(line: Line<'a>, query: &str, theme: crate::theme::Theme) -> Line<'a> {
    let query_lower = query.to_lowercase();
    let highlight_style = Style::default()
        .fg(theme.text_bright)
        .bg(theme.accent)
        .add_modifier(Modifier::BOLD);

    let mut new_spans: Vec<Span<'a>> = Vec::new();

    for span in line.spans {
        let text = &span.content;
        let text_lower = text.to_lowercase();
        let mut start = 0;

        loop {
            if let Some(pos) = text_lower[start..].find(&query_lower) {
                let abs_pos = start + pos;
                // Text before match
                if abs_pos > start {
                    new_spans.push(Span::styled(
                        text[start..abs_pos].to_string(),
                        span.style,
                    ));
                }
                // The match itself
                new_spans.push(Span::styled(
                    text[abs_pos..abs_pos + query.len()].to_string(),
                    highlight_style,
                ));
                start = abs_pos + query.len();
            } else {
                // Remainder
                if start < text.len() {
                    new_spans.push(Span::styled(
                        text[start..].to_string(),
                        span.style,
                    ));
                }
                break;
            }
        }
    }

    Line::from(new_spans)
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

fn draw_file_picker(f: &mut Frame, state: &AppState) {
    let theme = state.theme;
    let area = f.area();

    let height = area.height * 3 / 4;
    let width = (area.width * 3 / 4).max(50).min(area.width.saturating_sub(4));
    let popup = centered_rect(width, height, area);

    f.render_widget(Clear, popup);

    let dir_display = shorten_path(&state.browser.current_dir.display().to_string());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(format!(" {} ", dir_display))
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::vertical([
        Constraint::Length(1), // filter input
        Constraint::Min(1),   // file list
        Constraint::Length(1), // hint
    ])
    .split(inner);

    // Filter input
    let filter_line = if state.browser.filter.is_empty() {
        Line::from(Span::styled(
            " type to filter...",
            Style::default().fg(theme.text_muted),
        ))
    } else {
        Line::from(vec![
            Span::styled(" > ", Style::default().fg(theme.accent)),
            Span::styled(
                state.browser.filter.clone(),
                Style::default().fg(theme.text_bright),
            ),
        ])
    };
    f.render_widget(Paragraph::new(filter_line), chunks[0]);

    // File list
    let content_height = chunks[1].height as usize;
    let selected = state.browser.selected;

    let entries: Vec<_> = state.browser.filtered_entries()
        .into_iter()
        .enumerate()
        .map(|(i, (_idx, entry))| (entry, i == selected))
        .collect();

    let empty_msg = if state.browser.filter.is_empty() {
        "   No markdown files found"
    } else {
        "   No matches"
    };

    render_entry_list(
        &entries,
        theme,
        empty_msg,
        state.browser.scroll_offset,
        content_height,
        chunks[1],
        f,
    );

    let hint = Line::from(Span::styled(
        " enter:open  esc:close",
        Style::default().fg(theme.text_muted),
    ));
    f.render_widget(Paragraph::new(hint), chunks[2]);
}

fn draw_theme_picker(f: &mut Frame, state: &AppState) {
    let theme = state.theme;
    let area = f.area();

    let height = state.themes.len() as u16 + 4;
    let width = 38;
    let popup = centered_rect(width, height, area);

    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" Theme ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    let lines: Vec<Line> = state.themes
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            let is_selected = i == state.theme_index;
            let prefix = if is_selected { " > " } else { "   " };
            let style = if is_selected {
                Style::default()
                    .fg(theme.text_bright)
                    .bg(theme.cursor_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            let mut line = Line::from(Span::styled(format!("{}{}", prefix, name), style));
            if is_selected {
                let content_width: usize = line.spans.iter().map(|s| s.content.width()).sum();
                let area_width = chunks[0].width as usize;
                if content_width < area_width {
                    line.spans.push(Span::styled(
                        " ".repeat(area_width - content_width),
                        Style::default().bg(theme.cursor_bg),
                    ));
                }
            }
            line
        })
        .collect();

    f.render_widget(Paragraph::new(lines), chunks[0]);

    let hint = Line::from(Span::styled(
        " j/k:select  enter:ok  esc:cancel",
        Style::default().fg(theme.text_muted),
    ));
    f.render_widget(Paragraph::new(hint), chunks[1]);
}

fn draw_help(f: &mut Frame, state: &AppState) {
    let theme = state.theme;
    let area = f.area();

    let help_lines = vec![
        ("j / Down",     "Move cursor down / Select next"),
        ("k / Up",       "Move cursor up / Select previous"),
        ("Ctrl-f",       "Page down"),
        ("Ctrl-b",       "Page up"),
        ("g / Home",     "Go to top"),
        ("G / End",      "Go to bottom"),
        ("Enter",        "Fold/unfold section"),
        ("x / Space",    "Toggle task checkbox"),
        ("Ctrl-n / p",   "Next / previous unchecked task"),
        ("F",            "Toggle task filter view"),
        ("/",            "Search"),
        ("n / N",        "Next / previous match"),
        ("f",            "File picker"),
        ("e",            "Edit in $EDITOR"),
        ("t",            "Theme picker"),
        ("Tab / S-Tab",  "Next / previous tab"),
        ("?",            "Toggle help"),
        ("q",            "Close tab / quit"),
        ("Ctrl-c",       "Quit"),
    ];

    let max_content_width = help_lines
        .iter()
        .map(|(key, desc)| format!(" {:14}{}", key, desc).len())
        .max()
        .unwrap_or(0) as u16;
    let height = help_lines.len() as u16 + 4;
    let width = (max_content_width + 4).min(area.width.saturating_sub(4)); // +4 for borders + padding
    let popup = centered_rect(width, height, area);

    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" Help ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    let lines: Vec<Line> = help_lines
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(format!(" {:14}", key), Style::default().fg(theme.accent)),
                Span::styled(*desc, Style::default().fg(theme.text)),
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(lines), chunks[0]);

    let hint = Line::from(Span::styled(
        " esc/enter/?:close",
        Style::default().fg(theme.text_muted),
    ));
    f.render_widget(Paragraph::new(hint), chunks[1]);
}
