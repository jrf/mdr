use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};
use ratatui::Frame;

use crate::markdown::parse_markdown;
use crate::state::{AppMode, AppState};
use crate::theme::ALL_THEMES;

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
    match state.mode {
        AppMode::Browser => draw_browser(f, state),
        AppMode::Reader => draw_reader(f, state),
        AppMode::Search => draw_reader(f, state),
        AppMode::ThemePicker { .. } => {
            if let AppMode::ThemePicker { previous_mode, .. } = state.mode {
                match previous_mode {
                    crate::state::PreviousMode::Browser => draw_browser(f, state),
                    crate::state::PreviousMode::Reader => draw_reader(f, state),
                }
            }
            draw_theme_picker(f, state);
        }
        AppMode::Help { previous_mode } => {
            match previous_mode {
                crate::state::PreviousMode::Browser => draw_browser(f, state),
                crate::state::PreviousMode::Reader => draw_reader(f, state),
            }
            draw_help(f, state);
        }
    }
}

fn draw_browser(f: &mut Frame, state: &AppState) {
    let area = f.area();
    let theme = state.theme;

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .padding(Padding::horizontal(1));

    let inner = outer_block.inner(area);
    f.render_widget(outer_block, area);

    let chunks = Layout::vertical([
        Constraint::Length(1), // title bar
        Constraint::Length(1), // separator
        Constraint::Min(1),   // file list
        Constraint::Length(1), // status bar
    ])
    .split(inner);

    // Title bar
    let dir_display = shorten_path(&state.browser.current_dir.display().to_string());
    let title = Line::from(vec![
        Span::styled(
            "meld",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(theme.border)),
        Span::styled(dir_display, Style::default().fg(theme.text)),
    ]);
    f.render_widget(Paragraph::new(title), chunks[0]);

    // Separator
    let sep = "─".repeat(chunks[1].width as usize);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            sep,
            Style::default().fg(theme.border),
        ))),
        chunks[1],
    );

    // File list
    let content_area = chunks[2];
    let visible_height = content_area.height as usize;

    let entries = &state.browser.entries;
    let selected = state.browser.selected;
    let scroll = state.browser.scroll_offset;

    let lines: Vec<Line> = entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(i, entry)| {
            let is_selected = i == selected;
            let prefix = if is_selected { "> " } else { "  " };

            let icon = if entry.name == ".." {
                "^ "
            } else if entry.is_dir {
                "/ "
            } else {
                "  "
            };

            let style = if is_selected {
                Style::default()
                    .fg(theme.text_bright)
                    .add_modifier(Modifier::BOLD)
            } else if entry.name == ".." {
                Style::default().fg(theme.text_dim)
            } else if entry.is_dir {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.text)
            };

            Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(icon, style),
                Span::styled(&entry.name, style),
            ])
        })
        .collect();

    if lines.is_empty() {
        let empty = Line::from(Span::styled(
            "  No markdown files found",
            Style::default().fg(theme.text_dim),
        ));
        f.render_widget(Paragraph::new(vec![empty]), content_area);
    } else {
        f.render_widget(Paragraph::new(lines), content_area);
    }

    // Status bar
    let status = Line::from(vec![
        Span::styled(
            " ?:help",
            Style::default().fg(theme.text_muted),
        ),
    ]);
    f.render_widget(Paragraph::new(status), chunks[3]);
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

    let chunks = Layout::vertical([
        Constraint::Length(1), // title bar
        Constraint::Length(1), // separator
        Constraint::Min(1),   // content
        Constraint::Length(1), // status bar
    ])
    .split(inner);

    // Title bar
    let filename = state
        .file_path
        .as_ref()
        .map(|p| shorten_path(&p.display().to_string()))
        .unwrap_or_else(|| "no file".to_string());

    let title = Line::from(vec![
        Span::styled(
            "meld",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" │ ", Style::default().fg(theme.border)),
        Span::styled(filename, Style::default().fg(theme.text)),
    ]);
    f.render_widget(Paragraph::new(title), chunks[0]);

    // Separator
    let sep = "─".repeat(chunks[1].width as usize);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            sep,
            Style::default().fg(theme.border),
        ))),
        chunks[1],
    );

    // Content area
    let content_area = chunks[2];
    let styled_lines = parse_markdown(&state.content, theme, content_area.width);
    let total_lines = styled_lines.len();
    let visible_height = content_area.height as usize;

    state.total_lines = total_lines;
    state.visible_height = visible_height;

    let scroll = state.scroll.min(total_lines.saturating_sub(visible_height));

    let visible: Vec<Line> = styled_lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .map(|sl| {
            if state.search_query.is_empty() {
                sl.line
            } else {
                highlight_search(sl.line, &state.search_query, theme)
            }
        })
        .collect();

    f.render_widget(Paragraph::new(visible), content_area);

    // Status bar
    let is_searching = matches!(state.mode, AppMode::Search);

    if is_searching {
        let match_info = if state.search_matches.is_empty() {
            if state.search_query.is_empty() {
                String::new()
            } else {
                " (no matches)".to_string()
            }
        } else {
            format!(" ({}/{})", state.search_current + 1, state.search_matches.len())
        };

        let status = Line::from(vec![
            Span::styled("/", Style::default().fg(theme.accent)),
            Span::styled(
                state.search_query.clone(),
                Style::default().fg(theme.text_bright),
            ),
            Span::styled(match_info, Style::default().fg(theme.text_dim)),
        ]);
        f.render_widget(Paragraph::new(status), chunks[3]);
    } else {
        let scroll_pct = if total_lines <= visible_height {
            100
        } else {
            ((scroll as f64 / (total_lines - visible_height) as f64) * 100.0) as usize
        };

        let mut status_spans = vec![
            Span::styled(
                format!(" {}%", scroll_pct),
                Style::default().fg(theme.text_dim),
            ),
            Span::styled(" │ ", Style::default().fg(theme.border)),
            Span::styled(
                "?:help",
                Style::default().fg(theme.text_muted),
            ),
        ];

        if !state.search_query.is_empty() {
            status_spans.push(Span::styled(" │ ", Style::default().fg(theme.border)));
            let match_info = if state.search_matches.is_empty() {
                format!("/{}", state.search_query)
            } else {
                format!("/{} ({}/{})", state.search_query, state.search_current + 1, state.search_matches.len())
            };
            status_spans.push(Span::styled(match_info, Style::default().fg(theme.text_dim)));
        }

        f.render_widget(Paragraph::new(Line::from(status_spans)), chunks[3]);
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

fn draw_theme_picker(f: &mut Frame, state: &AppState) {
    let theme = state.theme;
    let area = f.area();

    let height = ALL_THEMES.len() as u16 + 4;
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

    let lines: Vec<Line> = ALL_THEMES
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            let is_selected = i == state.theme_index;
            let prefix = if is_selected { " > " } else { "   " };
            let style = if is_selected {
                Style::default()
                    .fg(theme.text_bright)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            Line::from(Span::styled(format!("{}{}", prefix, name), style))
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
        ("j / Down",     "Scroll down / Select next"),
        ("k / Up",       "Scroll up / Select previous"),
        ("Ctrl-f",       "Page down"),
        ("Ctrl-b",       "Page up"),
        ("g / Home",     "Go to top"),
        ("G / End",      "Go to bottom"),
        ("Enter",        "Open file"),
        ("/",            "Search"),
        ("n / N",        "Next / previous match"),
        ("Backspace",    "Back to browser"),
        ("e",            "Edit in $EDITOR"),
        ("t",            "Theme picker"),
        ("?",            "Toggle help"),
        ("q / Ctrl-c",   "Quit"),
    ];

    let height = help_lines.len() as u16 + 4;
    let width = 44;
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
