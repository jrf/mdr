use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::Frame;

use crate::markdown::parse_markdown;
use crate::state::{AppMode, AppState};
use crate::theme::ALL_THEMES;

pub fn draw(f: &mut Frame, state: &AppState) {
    match state.mode {
        AppMode::Browser => draw_browser(f, state),
        AppMode::Reader => draw_reader(f, state),
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
            "q:quit  j/k:nav  enter:open  t:theme",
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
            "q:quit  b:back  j/k:scroll  t:theme",
            Style::default().fg(theme.text_muted),
        ),
    ]);
    f.render_widget(Paragraph::new(status), chunks[3]);
}
