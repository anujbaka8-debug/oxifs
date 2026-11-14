use crate::app::{App, AppMode, Clipboard};
use crate::fs_ops::{human_size, FileEntry};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};

// Catppuccin Mocha
const BG: Color = Color::Rgb(30, 30, 46);
const FG: Color = Color::Rgb(205, 214, 244);
const ACCENT: Color = Color::Rgb(245, 194, 231);  // pink
const SEL_BG: Color = Color::Rgb(49, 50, 68);
const DIR_COLOR: Color = Color::Rgb(137, 180, 250); // blue
const LINK_COLOR: Color = Color::Rgb(166, 227, 161); // green
const MUTED: Color = Color::Rgb(108, 112, 134);
const RED: Color = Color::Rgb(243, 139, 168);

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Outer vertical split: main body + status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let body = chunks[0];
    let status_bar = chunks[1];

    // Horizontal split: file list | preview
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(body);

    draw_file_list(f, app, cols[0]);
    draw_preview(f, app, cols[1]);
    draw_status_bar(f, app, status_bar);

    // Overlay modals
    match &app.mode {
        AppMode::ConfirmDelete => draw_confirm(f, app, area),
        AppMode::Input { .. } => draw_input(f, app, area),
        AppMode::Search(_) => draw_search_bar(f, app, area),
        _ => {}
    }
}

fn draw_file_list(f: &mut Frame, app: &mut App, area: Rect) {
    let list_height = area.height.saturating_sub(2) as usize; // subtract borders

    let title = {
        let p = app.cwd.to_string_lossy();
        format!(" 󰉋 {p} ")
    };

    // Render only the visible slice; track selection within that slice
    let visible: Vec<ListItem> = app
        .entries
        .iter()
        .enumerate()
        .skip(app.scroll)
        .take(list_height)
        .map(|(i, e)| file_list_item(e, i, app))
        .collect();

    let selected_in_view = app.cursor.saturating_sub(app.scroll);
    let mut state = ratatui::widgets::ListState::default();
    if !app.entries.is_empty() {
        state.select(Some(selected_in_view));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(Span::styled(title, Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)))
        .style(Style::default().bg(BG).fg(FG));

    let list = List::new(visible)
        .block(block)
        .highlight_style(
            Style::default().bg(SEL_BG).fg(ACCENT).add_modifier(Modifier::BOLD),
        );

    f.render_stateful_widget(list, area, &mut state);
}

fn file_list_item<'a>(e: &'a FileEntry, idx: usize, app: &App) -> ListItem<'a> {
    let (icon, color) = if e.is_symlink {
        ("󰌷 ", LINK_COLOR)
    } else if e.is_dir {
        ("󰉋 ", DIR_COLOR)
    } else {
        (file_icon(&e.name), FG)
    };

    let in_search = matches!(&app.mode, AppMode::Search(_))
        && app.search_matches.contains(&idx);

    let style = if in_search {
        Style::default().fg(Color::Rgb(249, 226, 175)) // yellow highlight
    } else {
        Style::default().fg(color)
    };

    let clip_marker = match &app.clipboard {
        Clipboard::Copy(p) if *p == e.path => " [C]",
        Clipboard::Cut(p) if *p == e.path => " [X]",
        _ => "",
    };

    let size_str = if e.is_dir {
        "    ".into()
    } else {
        format!("{:>6}", human_size(e.size))
    };

    let line = Line::from(vec![
        Span::styled(format!("{icon}{}{clip_marker}", e.name), style),
        Span::styled(format!("  {size_str}"), Style::default().fg(MUTED)),
    ]);

    ListItem::new(line)
}

fn file_icon(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" => "󱘗 ",
        "toml" | "yaml" | "yml" => "󰬔 ",
        "md" => "󰍔 ",
        "sh" | "bash" | "zsh" | "fish" => "󰆍 ",
        "py" => "󰌠 ",
        "js" | "ts" => "󰌞 ",
        "json" => "󰘦 ",
        "png" | "jpg" | "jpeg" | "webp" | "svg" | "gif" => "󰋩 ",
        "mp4" | "mkv" | "webm" | "mov" => "󰎁 ",
        "mp3" | "flac" | "ogg" | "wav" => "󰝚 ",
        "zip" | "tar" | "gz" | "xz" | "bz2" | "zst" => "󰛫 ",
        "pdf" => "󰈦 ",
        "txt" => "󰈙 ",
        "html" | "htm" => "󰌾 ",
        "css" => "󰌜 ",
        _ => "󰈚 ",
    }
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MUTED))
        .title(Span::styled(" Preview ", Style::default().fg(MUTED)))
        .style(Style::default().bg(BG).fg(FG));

    let content: Vec<Line> = match app.selected_entry() {
        None => vec![Line::from(Span::styled("(empty)", Style::default().fg(MUTED)))],
        Some(e) => {
            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Name: ", Style::default().fg(ACCENT)),
                    Span::styled(e.name.clone(), Style::default().fg(FG)),
                ]),
                Line::from(vec![
                    Span::styled("Path: ", Style::default().fg(ACCENT)),
                    Span::styled(
                        e.path.to_string_lossy().into_owned(),
                        Style::default().fg(MUTED),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Type: ", Style::default().fg(ACCENT)),
                    Span::styled(
                        if e.is_symlink {
                            "Symlink"
                        } else if e.is_dir {
                            "Directory"
                        } else {
                            "File"
                        },
                        Style::default().fg(FG),
                    ),
                ]),
            ];
            if !e.is_dir {
                lines.push(Line::from(vec![
                    Span::styled("Size: ", Style::default().fg(ACCENT)),
                    Span::styled(human_size(e.size), Style::default().fg(FG)),
                ]));
            }
            if let Some(mtime) = &e.modified {
                lines.push(Line::from(vec![
                    Span::styled("Modified: ", Style::default().fg(ACCENT)),
                    Span::styled(
                        mtime.format("%Y-%m-%d %H:%M").to_string(),
                        Style::default().fg(FG),
                    ),
                ]));
            }

            // Text preview for small files
            if !e.is_dir && e.size < 32_768 {
                if let Ok(text) = std::fs::read_to_string(&e.path) {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled("─── Preview ───", Style::default().fg(MUTED))));
                    for l in text.lines().take((area.height as usize).saturating_sub(8)) {
                        lines.push(Line::from(Span::styled(l.to_owned(), Style::default().fg(FG))));
                    }
                }
            }

            lines
        }
    };

    let para = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });

    f.render_widget(para, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let (mode_str, mode_color) = match &app.mode {
        AppMode::Normal => (" NORMAL ", ACCENT),
        AppMode::Loading => (" LOADING ", Color::Rgb(249, 226, 175)),
        AppMode::Search(_) => (" SEARCH ", Color::Rgb(166, 227, 161)),
        AppMode::ConfirmDelete => (" DELETE? ", RED),
        AppMode::Input { action, .. } => match action {
            crate::app::InputAction::Rename => (" RENAME ", Color::Rgb(203, 166, 247)),
            crate::app::InputAction::Mkdir => (" MKDIR ", Color::Rgb(203, 166, 247)),
        },
    };

    let count = format!(" {}/{} ", app.cursor + 1, app.entries.len().max(1));
    let status = if app.status.is_empty() {
        " q:quit  h:up  j↓  k↑  Enter:open  d:trash  y:copy  x:cut  p:paste  r:rename  m:mkdir  .:hidden  /:search  Y:yank-path ".to_string()
    } else {
        format!(" {} ", app.status)
    };

    let left = Span::styled(mode_str, Style::default().bg(mode_color).fg(BG).add_modifier(Modifier::BOLD));
    let mid = Span::styled(status, Style::default().bg(SEL_BG).fg(FG));
    let right = Span::styled(count, Style::default().bg(MUTED).fg(BG));

    let line = Line::from(vec![left, mid, right]);
    let para = Paragraph::new(line).style(Style::default().bg(BG));
    f.render_widget(para, area);
}

fn draw_confirm(f: &mut Frame, app: &App, area: Rect) {
    let name = app.selected_entry().map(|e| e.name.as_str()).unwrap_or("?");
    let msg = format!(" Trash '{name}'? [y] confirm  [n/Esc] cancel ");
    let w = (msg.len() + 4).min(area.width as usize) as u16;
    let h = 3u16;
    let popup = centered_rect(w, h, area);
    let para = Paragraph::new(msg)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(RED)))
        .style(Style::default().bg(BG).fg(RED));
    f.render_widget(para, popup);
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    if let AppMode::Input { prompt, value, .. } = &app.mode {
        let display = format!(" {prompt} {value}_ ");
        let w = (display.len() + 4).min(area.width as usize) as u16;
        let h = 3u16;
        let popup = centered_rect(w, h, area);
        let para = Paragraph::new(display)
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(ACCENT)))
            .style(Style::default().bg(BG).fg(FG));
        f.render_widget(para, popup);
    }
}

fn draw_search_bar(f: &mut Frame, app: &App, area: Rect) {
    if let AppMode::Search(q) = &app.mode {
        let matches = app.search_matches.len();
        let display = format!(" / {q}  ({matches} matches) ");
        let popup = Rect {
            x: 0,
            y: area.height.saturating_sub(2),
            width: area.width,
            height: 1,
        };
        let para = Paragraph::new(display)
            .style(Style::default().bg(SEL_BG).fg(Color::Rgb(166, 227, 161)));
        f.render_widget(para, popup);
    }
}

fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    Rect { x, y, width: w.min(area.width), height: h.min(area.height) }
}
