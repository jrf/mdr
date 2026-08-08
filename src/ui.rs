use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::markdown::tag_color;
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
        AppMode::FilterPicker { .. } => draw_filter_picker(f, state),
        AppMode::TableOfContents { .. } => draw_toc(f, state),
        AppMode::BookmarkList { .. } => draw_bookmark_list(f, state),
        AppMode::Help => draw_help(f, state),
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
            .unwrap_or_else(|| "[stdin]".to_string());

        if i == state.active_tab {
            spans.push(Span::styled(
                format!(" {} ", name),
                Style::default()
                    .fg(theme.selection)
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

    // Content area — split into gutter + body
    let full_content_area = if show_tabs { chunks[2] } else { chunks[0] };
    let status_area = if show_tabs { chunks[3] } else { chunks[1] };

    let gutter_width = 2u16;
    let content_split = Layout::horizontal([
        Constraint::Length(gutter_width),
        Constraint::Min(1),
    ])
    .split(full_content_area);
    let gutter_area = content_split[0];
    let content_area = content_split[1];

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
    let cursor_source_line = display_indices
        .get(cursor)
        .and_then(|&idx| tab.cached_lines.get(idx).and_then(|sl| sl.source_line));

    // Build gutter lines
    let gutter_lines: Vec<Line> = display_indices[scroll..]
        .iter()
        .enumerate()
        .take(visible_height)
        .map(|(i, &line_idx)| {
            let sl = &tab.cached_lines[line_idx];
            let is_bookmarked = tab.bookmarks.contains(&(scroll + i));

            if sl.is_heading && sl.heading_text.is_some() {
                let indicator = if tab.folded_headings.contains(&line_idx) { "▶" } else { "▼" };
                return Line::from(Span::styled(
                    format!("{} ", indicator),
                    Style::default().fg(theme.text_dim),
                ));
            }
            if is_bookmarked {
                return Line::from(Span::styled(
                    "● ",
                    Style::default().fg(theme.accent),
                ));
            }
            Line::from("  ")
        })
        .collect();

    f.render_widget(Paragraph::new(gutter_lines), gutter_area);

    // Build content lines
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
            // Highlight cursor line with a subtle background. For wrapped task
            // items, also highlight sibling lines that share the same source line
            // so the whole logical item is visually selected.
            let is_cursor_line = scroll + i == cursor;
            let is_wrap_companion = !is_cursor_line
                && cursor_source_line.is_some()
                && sl.source_line == cursor_source_line;
            if is_cursor_line || is_wrap_companion {
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
            Span::styled("/", Style::default().fg(theme.key)),
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
                Style::default().fg(theme.key),
            ),
        ];

        if tab.filter_tasks {
            status_spans.push(Span::styled(" │ ", Style::default().fg(theme.border)));
            status_spans.push(Span::styled(
                "[tasks]",
                Style::default().fg(theme.labels.features).add_modifier(Modifier::BOLD),
            ));
        }

        if let Some(ref tag) = tab.tag_filter {
            status_spans.push(Span::styled(" │ ", Style::default().fg(theme.border)));
            status_spans.push(Span::styled(
                format!("#{}",tag),
                Style::default().fg(tag_color(tag, &theme)).add_modifier(Modifier::BOLD),
            ));
        }

        if tab.file_updated {
            status_spans.push(Span::styled(" │ ", Style::default().fg(theme.border)));
            status_spans.push(Span::styled(
                "[updated]",
                Style::default().fg(theme.labels.features).add_modifier(Modifier::BOLD),
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
        .fg(theme.background_dark)
        .bg(theme.selection)
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

fn picker_rect(area: Rect) -> Rect {
    let width = if area.width > 4 {
        (area.width * 3 / 4).max(50).min(area.width - 4)
    } else {
        area.width.max(1)
    };
    let height = if area.height > 4 {
        (area.height * 3 / 4).max(6).min(area.height - 2)
    } else {
        area.height.max(1)
    };
    centered_rect(width, height, area)
}

fn draw_file_picker(f: &mut Frame, state: &AppState) {
    let theme = state.theme;
    let area = f.area();

    let popup = picker_rect(area);

    f.render_widget(Clear, area);
    f.render_widget(
        Block::default().style(Style::default().bg(theme.background_deep)),
        area,
    );
    f.render_widget(Clear, popup);

    let dir_display = shorten_path(&state.browser.current_dir.display().to_string());
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(theme.text).bg(theme.background))
        .border_style(Style::default().fg(theme.picker_border))
        .title(format!(" {} ", dir_display))
        .title_style(
            Style::default()
                .fg(theme.picker_accent)
                .bg(theme.background)
                .add_modifier(Modifier::BOLD),
        );

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
            Style::default()
                .fg(theme.text_dim)
                .bg(theme.background_dark),
        ))
    } else {
        Line::from(vec![
            Span::styled(
                " > ",
                Style::default()
                    .fg(theme.picker_accent)
                    .bg(theme.background_dark),
            ),
            Span::styled(
                state.browser.filter.clone(),
                Style::default()
                    .fg(theme.text)
                    .bg(theme.background_dark),
            ),
        ])
    };
    f.render_widget(
        Paragraph::new(filter_line).style(Style::default().bg(theme.background_dark)),
        chunks[0],
    );

    // File list
    let content_height = chunks[1].height as usize;
    let entries = state.browser.filtered_entries();
    let recent_heading_index = state.browser.recent_heading_index();
    let mut lines = Vec::with_capacity(content_height);
    for (index, (_, entry)) in entries
        .iter()
        .enumerate()
        .skip(state.browser.scroll_offset)
    {
        if Some(index) == recent_heading_index && lines.len() + 1 < content_height {
            lines.push(picker_recent_heading_line(
                chunks[1].width as usize,
                theme,
            ));
        }
        if lines.len() >= content_height {
            break;
        }
        lines.push(picker_entry_line(
            entry,
            &state.browser,
            index == state.browser.selected,
            chunks[1].width as usize,
            theme,
        ));
        if lines.len() >= content_height {
            break;
        }
    }
    if lines.is_empty() {
        let message = if state.browser.filter.is_empty() {
            "   No markdown files found"
        } else {
            "   No matches"
        };
        lines.push(Line::from(Span::styled(
            message,
            Style::default()
                .fg(theme.text_dim)
                .bg(theme.background),
        )));
    }
    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.background)),
        chunks[1],
    );

    let status = if state.browser.recursive_loading() {
        Some((
            format!("scanning • {} shown", entries.len()),
            theme.picker_loading,
        ))
    } else {
        let position = if entries.is_empty() {
            0
        } else {
            state.browser.selected + 1
        };
        Some((format!("{position}/{}", entries.len()), theme.text_dim))
    };
    f.render_widget(
        Paragraph::new(picker_hint_line(
            &[("enter", "open"), ("esc", "close")],
            status,
            theme,
        ))
        .style(Style::default().bg(theme.background_dark)),
        chunks[2],
    );
}

fn picker_recent_heading_line(width: usize, theme: crate::theme::Theme) -> Line<'static> {
    let label = " Most Recent ";
    let mut spans = vec![
        Span::styled("  ", Style::default().bg(theme.background)),
        Span::styled(
            label,
            Style::default()
                .fg(theme.picker_recent)
                .bg(theme.background)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    let used = 2 + label.chars().count();
    if used < width {
        spans.push(Span::styled(
            "─".repeat(width - used),
            Style::default()
                .fg(theme.picker_border)
                .bg(theme.background),
        ));
    }
    Line::from(spans)
}

fn truncate_left(value: &str, max_width: usize) -> String {
    if value.width() <= max_width {
        return value.to_string();
    }
    if max_width == 0 {
        return String::new();
    }

    let suffix_width = max_width.saturating_sub(1);
    let mut suffix = Vec::new();
    let mut used = 0;
    for character in value.chars().rev() {
        let width = character.to_string().width();
        if used + width > suffix_width {
            break;
        }
        suffix.push(character);
        used += width;
    }
    suffix.reverse();
    format!("…{}", suffix.into_iter().collect::<String>())
}

fn picker_entry_line(
    entry: &crate::browser::BrowserEntry,
    browser: &crate::browser::BrowserState,
    selected: bool,
    width: usize,
    theme: crate::theme::Theme,
) -> Line<'static> {
    let background = if selected {
        theme.cursor_bg
    } else {
        theme.background
    };
    let marker_style = Style::default().fg(theme.picker_accent).bg(background);
    let icon = if entry.name == "../" {
        "↑ "
    } else if entry.is_dir {
        "› "
    } else {
        "  "
    };
    let icon_color = if entry.name == "../" {
        theme.text_dim
    } else if entry.is_dir {
        theme.picker_directory
    } else if entry.is_recent {
        theme.picker_recent
    } else {
        theme.text
    };
    let mut spans = vec![
        Span::styled(if selected { "▌ " } else { "  " }, marker_style),
        Span::styled(icon, Style::default().fg(icon_color).bg(background)),
    ];

    let matches = browser.match_indices(&entry.name);
    let basename_start = if entry.is_dir {
        0
    } else {
        entry
            .name
            .char_indices()
            .rev()
            .find(|(_, character)| *character == '/')
            .map_or(0, |(index, _)| entry.name[..=index].chars().count())
    };
    for (index, character) in entry.name.chars().enumerate() {
        let foreground = if matches.binary_search(&index).is_ok() {
            theme.picker_matched
        } else if index < basename_start || entry.name == "../" {
            theme.text_dim
        } else if entry.is_recent {
            theme.picker_recent
        } else if entry.is_dir {
            theme.picker_directory
        } else {
            theme.text
        };
        let mut style = Style::default().fg(foreground).bg(background);
        if matches.binary_search(&index).is_ok() || (selected && index >= basename_start) {
            style = style.add_modifier(Modifier::BOLD);
        }
        spans.push(Span::styled(character.to_string(), style));
    }

    let mut used: usize = spans.iter().map(|span| span.content.width()).sum();
    let recent_parent = if entry.is_recent && browser.filter.is_empty() {
        entry.path.parent()
    } else {
        None
    };
    if let Some(parent) = recent_parent {
        let parent = shorten_path(&parent.to_string_lossy());
        let available = width.saturating_sub(used + 2);
        if available >= 3 {
            let parent = truncate_left(&parent, available);
            let parent_width = parent.width();
            let gap = width.saturating_sub(used + parent_width);
            spans.push(Span::styled(
                " ".repeat(gap),
                Style::default().bg(background),
            ));
            spans.push(Span::styled(
                parent,
                Style::default().fg(theme.text_dim).bg(background),
            ));
            used = width;
        }
    }
    if used < width {
        spans.push(Span::styled(
            " ".repeat(width - used),
            Style::default().bg(background),
        ));
    }
    Line::from(spans)
}

fn picker_hint_line(
    bindings: &[(&str, &str)],
    status: Option<(String, Color)>,
    theme: crate::theme::Theme,
) -> Line<'static> {
    let mut spans = vec![Span::styled(
        " ",
        Style::default().bg(theme.background_dark),
    )];
    for (key, action) in bindings {
        spans.push(Span::styled(
            format!(" {key} "),
            Style::default()
                .fg(theme.picker_accent)
                .bg(theme.cursor_bg)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {action}  "),
            Style::default()
                .fg(theme.text_dim)
                .bg(theme.background_dark),
        ));
    }
    if let Some((status, color)) = status {
        spans.push(Span::styled(
            status,
            Style::default().fg(color).bg(theme.background_dark),
        ));
    }
    Line::from(spans)
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

    let visible_rows = chunks[0].height as usize;
    let first_visible = state
        .theme_index
        .saturating_add(1)
        .saturating_sub(visible_rows);
    let lines: Vec<Line> = state.themes
        .iter()
        .enumerate()
        .skip(first_visible)
        .take(visible_rows)
        .map(|(i, (name, _))| {
            let is_selected = i == state.theme_index;
            let prefix = if is_selected { " > " } else { "   " };
            let style = if is_selected {
                Style::default()
                    .fg(theme.selection)
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
        Style::default().fg(theme.key),
    ));
    f.render_widget(Paragraph::new(hint), chunks[1]);
}

fn draw_filter_picker(f: &mut Frame, state: &AppState) {
    let (picker, filter) = match &state.mode {
        AppMode::FilterPicker { picker, filter } => (picker, filter),
        _ => return,
    };
    let theme = state.theme;
    let area = f.area();
    let options = &picker.items;

    let height = options.len() as u16 + 6; // +2 for filter input + spacer
    let width = 34;
    let popup = centered_rect(width, height, area);

    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" Labels ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::vertical([
        Constraint::Length(1), // filter input
        Constraint::Min(1),   // list
        Constraint::Length(1), // hint
    ])
    .split(inner);

    // Filter input
    let filter_line = if filter.is_empty() {
        Line::from(Span::styled(
            " type to filter...",
            Style::default().fg(theme.text_muted),
        ))
    } else {
        Line::from(vec![
            Span::styled(" > ", Style::default().fg(theme.key)),
            Span::styled(filter.clone(), Style::default().fg(theme.text_bright)),
        ])
    };
    f.render_widget(Paragraph::new(filter_line), chunks[0]);

    let lines: Vec<Line> = options
        .iter()
        .enumerate()
        .map(|(i, option)| {
            let is_selected = i == picker.selected;
            let prefix = if is_selected { " > " } else { "   " };
            let display = if option == "None" {
                "None".to_string()
            } else {
                format!("#{}", option)
            };
            let fg = if option == "None" {
                theme.text
            } else {
                crate::markdown::tag_color(option, &theme)
            };
            let style = if is_selected {
                Style::default()
                    .fg(fg)
                    .bg(theme.cursor_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(fg)
            };
            let mut line = Line::from(Span::styled(format!("{}{}", prefix, display), style));
            if is_selected {
                let content_width: usize = line.spans.iter().map(|s| s.content.width()).sum();
                let area_width = chunks[1].width as usize;
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

    f.render_widget(Paragraph::new(lines), chunks[1]);

    let hint = Line::from(Span::styled(
        " enter:ok  esc:cancel",
        Style::default().fg(theme.key),
    ));
    f.render_widget(Paragraph::new(hint), chunks[2]);
}

fn draw_toc(f: &mut Frame, state: &AppState) {
    let picker = match &state.mode {
        AppMode::TableOfContents { picker } => picker,
        _ => return,
    };
    let theme = state.theme;
    let area = f.area();
    let entries = &picker.items;

    let max_text_width = entries.iter()
        .map(|(text, _, level)| {
            let indent = (*level as usize).saturating_sub(1) * 2;
            indent + text.len() + 4
        })
        .max()
        .unwrap_or(20);
    let width = (max_text_width as u16 + 4).min(area.width.saturating_sub(4)).max(30);
    let height = (entries.len() as u16 + 4).min(area.height.saturating_sub(4));
    let popup = centered_rect(width, height, area);

    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" Outline ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    let visible_height = chunks[0].height as usize;
    let scroll = if picker.selected < picker.scroll {
        picker.selected
    } else if picker.selected >= picker.scroll + visible_height {
        picker.selected.saturating_sub(visible_height - 1)
    } else {
        picker.scroll
    };

    let lines: Vec<Line> = entries
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(i, (text, _, level))| {
            let is_selected = i == picker.selected;
            let indent = " ".repeat((*level as usize).saturating_sub(1) * 2);
            let prefix = if is_selected { " > " } else { "   " };
            let color = match level {
                1 => theme.heading,
                2 => theme.heading,
                _ => theme.text_bright,
            };
            let style = if is_selected {
                Style::default()
                    .fg(theme.selection)
                    .bg(theme.cursor_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };
            let mut line = Line::from(Span::styled(format!("{}{}{}", prefix, indent, text), style));
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
        " j/k:select  enter:jump  esc/o:cancel",
        Style::default().fg(theme.key),
    ));
    f.render_widget(Paragraph::new(hint), chunks[1]);
}

fn draw_bookmark_list(f: &mut Frame, state: &AppState) {
    let picker = match &state.mode {
        AppMode::BookmarkList { picker } => picker,
        _ => return,
    };
    let theme = state.theme;
    let area = f.area();

    let height = (picker.items.len() as u16 + 4).min(area.height.saturating_sub(4));
    let width = 56u16.min(area.width.saturating_sub(4));
    let popup = centered_rect(width, height, area);

    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(" Bookmarks ")
        .title_style(Style::default().fg(theme.accent).add_modifier(Modifier::BOLD));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    let visible_height = chunks[0].height as usize;
    let scroll = if picker.selected < picker.scroll {
        picker.selected
    } else if picker.selected >= picker.scroll + visible_height {
        picker.selected.saturating_sub(visible_height - 1)
    } else {
        picker.scroll
    };

    let lines: Vec<Line> = picker.items.iter()
        .enumerate()
        .skip(scroll)
        .take(visible_height)
        .map(|(i, (_pos, text))| {
            let is_selected = i == picker.selected;
            let prefix = if is_selected { " > " } else { "   " };
            let style = if is_selected {
                Style::default()
                    .fg(theme.selection)
                    .bg(theme.cursor_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            let mut line = Line::from(Span::styled(format!("{}{}", prefix, text), style));
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
        " j/k:select  enter:jump  esc/B:cancel",
        Style::default().fg(theme.key),
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
        ("Enter",        "Fold/unfold / follow link"),
        ("[ / ]",        "Fold all / unfold all"),
        ("x / Space",    "Toggle task checkbox"),
        ("Ctrl-n / p",   "Next / previous unchecked task"),
        ("u",            "Toggle unchecked task filter"),
        ("l",            "Filter by label"),
        ("o",            "Outline / table of contents"),
        ("b",            "Toggle bookmark"),
        ("B",            "Bookmark list"),
        ("' / \"",       "Next / previous bookmark"),
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
                Span::styled(format!(" {:14}", key), Style::default().fg(theme.key)),
                Span::styled(*desc, Style::default().fg(theme.text)),
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(lines), chunks[0]);

    let hint = Line::from(Span::styled(
        " esc/enter/?:close",
        Style::default().fg(theme.key),
    ));
    f.render_widget(Paragraph::new(hint), chunks[1]);
}

#[cfg(test)]
mod tests {
    use super::{draw, picker_rect};
    use crate::browser::BrowserEntry;
    use crate::state::{AppMode, AppState, Tab};
    use crate::theme::default_theme;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::Terminal;
    use std::path::PathBuf;

    fn find_text(buffer: &Buffer, needle: &str) -> Option<(u16, u16)> {
        let chars: Vec<String> = needle.chars().map(|ch| ch.to_string()).collect();
        for y in buffer.area.y..buffer.area.y + buffer.area.height {
            for x in buffer.area.x..buffer.area.x + buffer.area.width {
                if x + chars.len() as u16 > buffer.area.x + buffer.area.width {
                    break;
                }
                if chars
                    .iter()
                    .enumerate()
                    .all(|(offset, ch)| buffer[(x + offset as u16, y)].symbol() == ch)
                {
                    return Some((x, y));
                }
            }
        }
        None
    }

    #[test]
    fn reader_uses_selection_and_key_roles() {
        let theme = default_theme();
        let mut state = AppState::new_reader(
            PathBuf::from("synthetic-one.md"),
            "# Synthetic".into(),
            0,
            vec![("test".into(), theme)],
            false,
        );
        state.tabs.push(Tab::new(
            PathBuf::from("synthetic-two.md"),
            "# Synthetic".into(),
            theme,
        ));
        let mut terminal = Terminal::new(TestBackend::new(100, 32)).expect("test terminal");

        terminal
            .draw(|frame| draw(frame, &mut state))
            .expect("draw reader");
        let buffer = terminal.backend().buffer();
        let (tab_x, tab_y) = find_text(buffer, "synthetic-one.md").expect("active tab");
        assert_eq!(buffer[(tab_x, tab_y)].fg, theme.selection);

        state.mode = AppMode::Help;
        terminal
            .draw(|frame| draw(frame, &mut state))
            .expect("draw help");
        let buffer = terminal.backend().buffer();
        let (key_x, key_y) = find_text(buffer, "j / Down").expect("help key");
        assert_eq!(buffer[(key_x, key_y)].fg, theme.key);
    }

    #[test]
    fn theme_picker_scrolls_to_selected_theme() {
        let theme = default_theme();
        let themes = (0..16)
            .map(|index| (format!("theme {index:02}"), theme))
            .collect();
        let mut state = AppState::new_reader(
            PathBuf::from("synthetic.md"),
            "# Synthetic".into(),
            15,
            themes,
            false,
        );
        state.mode = AppMode::ThemePicker { original_index: 15 };
        let mut terminal = Terminal::new(TestBackend::new(60, 10)).expect("test terminal");

        terminal
            .draw(|frame| draw(frame, &mut state))
            .expect("draw theme picker");

        assert!(find_text(terminal.backend().buffer(), "theme 15").is_some());
    }

    #[test]
    fn file_picker_uses_layered_theme_and_selection_marker() {
        let theme = default_theme();
        let mut state = AppState::new_picker(
            PathBuf::from("synthetic"),
            0,
            vec![("test".into(), theme)],
            false,
        );
        state.browser.entries = vec![BrowserEntry {
            name: "notes.md".into(),
            path: PathBuf::from("notes.md"),
            is_dir: false,
            is_recent: false,
        }];
        state.browser.filtered_indices = vec![0];
        let area = Rect::new(0, 0, 80, 30);
        let popup = picker_rect(area);
        let mut terminal =
            Terminal::new(TestBackend::new(area.width, area.height)).expect("test terminal");

        terminal
            .draw(|frame| draw(frame, &mut state))
            .expect("draw picker");
        let buffer = terminal.backend().buffer();

        assert_eq!(buffer[(0, 0)].symbol(), " ");
        assert_eq!(buffer[(0, 0)].bg, theme.background_deep);
        assert_eq!(buffer[(popup.x, popup.y)].fg, theme.picker_border);
        assert_eq!(buffer[(popup.x + 1, popup.y + 1)].bg, theme.background_dark);
        assert_eq!(buffer[(popup.x + 1, popup.y + 2)].symbol(), "▌");
        assert_eq!(buffer[(popup.x + 1, popup.y + 2)].bg, theme.cursor_bg);
    }

    #[test]
    fn file_picker_labels_recent_documents_with_parent_directory() {
        let theme = default_theme();
        let mut state = AppState::new_picker(
            PathBuf::from("synthetic"),
            0,
            vec![("test".into(), theme)],
            false,
        );
        state.browser.entries = vec![BrowserEntry {
            name: "recent.md".into(),
            path: PathBuf::from("/synthetic/project/recent.md"),
            is_dir: false,
            is_recent: true,
        }];
        state.browser.filtered_indices = vec![0];
        let area = Rect::new(0, 0, 80, 30);
        let popup = picker_rect(area);
        let mut terminal =
            Terminal::new(TestBackend::new(area.width, area.height)).expect("test terminal");

        terminal
            .draw(|frame| draw(frame, &mut state))
            .expect("draw picker");
        let buffer = terminal.backend().buffer();
        let rendered: String = (popup.y + 1..popup.y + popup.height - 1)
            .flat_map(|y| {
                (popup.x + 1..popup.x + popup.width - 1)
                    .map(move |x| buffer[(x, y)].symbol().to_string())
            })
            .collect();

        assert!(rendered.contains("Most Recent"));
        assert!(rendered.contains("/synthetic/project"));
        let (path_x, path_y) = find_text(buffer, "/synthetic/project").expect("recent parent");
        assert_eq!(buffer[(path_x, path_y)].fg, theme.text_dim);
    }
}
