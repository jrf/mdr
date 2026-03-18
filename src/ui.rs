use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};
use ratatui::Frame;

use crate::markdown::parse_markdown;
use crate::state::{AppMode, AppState};
use crate::theme::ALL_THEMES;

pub fn draw(f: &mut Frame, state: &AppState) {
    match state.mode {
        AppMode::Browser => draw_browser(f, state),
        AppMode::Reader => draw_reader(f, state),
        AppMode::ThemePicker { .. } => {
            // Draw the underlying mode first, then overlay
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
    let dir_display = state.browser.current_dir.display().to_string();
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
    let theme_name = ALL_THEMES
        .get(state.theme_index)
        .map(|(name, _)| *name)
        .unwrap_or("unknown");

    let status = Line::from(vec![
        Span::styled(
            format!(" {} ", theme_name),
            Style::default().fg(theme.accent),
        ),
        Span::styled(" │ ", Style::default().fg(theme.border)),
        Span::styled(
            "q:quit  j/k:nav  enter:open  t:theme  ?:help",
            Style::default().fg(theme.text_muted),
        ),
    ]);
    f.render_widget(Paragraph::new(status), chunks[3]);
}

fn draw_reader(f: &mut Frame, state: &AppState) {
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
        .map(|p| p.display().to_string())
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
    let scroll = state.scroll.min(total_lines.saturating_sub(visible_height));

    let visible: Vec<Line> = styled_lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .map(|sl| sl.line)
        .collect();

    f.render_widget(Paragraph::new(visible), content_area);

    // Status bar
    let theme_name = ALL_THEMES
        .get(state.theme_index)
        .map(|(name, _)| *name)
        .unwrap_or("unknown");

    let scroll_pct = if total_lines <= visible_height {
        100
    } else {
        ((scroll as f64 / (total_lines - visible_height) as f64) * 100.0) as usize
    };

    let status = Line::from(vec![
        Span::styled(
            format!(" {} ", theme_name),
            Style::default().fg(theme.accent),
        ),
        Span::styled(" │ ", Style::default().fg(theme.border)),
        Span::styled(
            format!("{}%", scroll_pct),
            Style::default().fg(theme.text_dim),
        ),
        Span::styled(" │ ", Style::default().fg(theme.border)),
        Span::styled(
            "q:quit  bksp:back  j/k:scroll  t:theme  ?:help",
            Style::default().fg(theme.text_muted),
        ),
    ]);
    f.render_widget(Paragraph::new(status), chunks[3]);
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
    let width = 30;
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
        ("Ctrl-d",       "Page down"),
        ("Ctrl-u",       "Page up"),
        ("g / Home",     "Go to top"),
        ("G / End",      "Go to bottom"),
        ("Enter",        "Open file"),
        ("Backspace",    "Back to browser"),
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
