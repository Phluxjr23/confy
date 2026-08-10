use crate::app::{App, Item};
use crate::commands::{self, CmdResult};
use crate::picker::Picker;
use crate::tutorial;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState, Wrap},
    Terminal,
};
use std::path::Path;

#[derive(Default, PartialEq)]
enum Mode { #[default] Normal, Command, Search }

fn resolve_color(s: &str) -> Color {
    match s.to_lowercase().as_str() {
        "black"   => Color::Black,
        "red"     => Color::Red,
        "green"   => Color::Green,
        "yellow"  => Color::Yellow,
        "blue"    => Color::Blue,
        "magenta" | "pink" | "purple" => Color::Magenta,
        "cyan" | "lavender"           => Color::Cyan,
        "white"   => Color::White,
        "default" => Color::Reset,
        hex if hex.starts_with('#') && hex.len() == 7 => {
            let r = u8::from_str_radix(&hex[1..3], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[3..5], 16).unwrap_or(255);
            let b = u8::from_str_radix(&hex[5..7], 16).unwrap_or(255);
            Color::Rgb(r, g, b)
        }
        _ => Color::Reset,
    }
}

struct Colors { normal: Style, highlight: Style, group: Style, error: Style }

impl Colors {
    fn from_app(app: &App) -> Self {
        let c = &app.cfg.settings.colors;
        let fg = resolve_color(&c.fg);
        let bg = resolve_color(&c.bg);
        let hi = resolve_color(&c.highlight);
        let gr = resolve_color(&c.group);
        Self {
            normal:    Style::default().fg(fg).bg(bg),
            highlight: Style::default().fg(hi).bg(bg).add_modifier(Modifier::REVERSED),
            group:     Style::default().fg(gr).bg(bg).add_modifier(Modifier::BOLD),
            error:     Style::default().fg(Color::Red).bg(bg),
        }
    }
}

pub fn run<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let mut mode       = Mode::Normal;
    let mut cmd_buf    = String::new();
    let mut search_buf = String::new();
    let mut table_state = TableState::default();

    if app.show_tutorial {
        run_tutorial(terminal, app)?;
        app.show_tutorial = false;
        app.cfg.settings.first_startup = false;
        app.save();
    }

    loop {
        table_state.select(if app.flat_view.is_empty() { None } else { Some(app.selected) });

        terminal.draw(|f| {
            let colors = Colors::from_app(app);
            draw_main::<B>(f, app, &colors, &mode, &cmd_buf, &search_buf, &mut table_state);
        })?;

        if app.popup.is_some() {
            terminal.draw(|f| {
                let colors = Colors::from_app(app);
                draw_main::<B>(f, app, &colors, &mode, &cmd_buf, &search_buf, &mut table_state);
                draw_popup(f, app.popup.as_deref().unwrap_or(""));
            })?;
            event::read()?;
            app.popup = None;
            continue;
        }

        let Event::Key(key) = event::read()? else { continue };
        if key.kind != KeyEventKind::Press { continue; }

        match mode {
            Mode::Command => match key.code {
                KeyCode::Enter => {
                    let raw = cmd_buf.clone();
                    cmd_buf.clear();
                    mode = Mode::Normal;
                    match commands::handle(app, &raw) {
                        CmdResult::Quit => break,
                        CmdResult::NeedPicker { group } => {
                            drop_alt(terminal)?;
                            let browse = app.local_path(&app.cfg.config_dir).to_string_lossy().into_owned();
                            let mut picker = Picker::new(&browse, false);
                            if let Some(picked) = picker.run(terminal)? {
                                let tracked = app.tracked_path(&picked);
                                app.track_file(&std::path::PathBuf::from(&tracked), &group);
                            }
                            restore_alt(terminal)?;
                        }
                        CmdResult::NeedCdPicker => {
                            drop_alt(terminal)?;
                            let mut picker = Picker::new(&app.cfg.config_dir, true);
                            if let Some(dir) = picker.run(terminal)? {
                                app.cfg.config_dir = dir.to_string_lossy().into_owned();
                                app.save();
                                app.popup = Some(format!("config dir → {}", app.cfg.config_dir));
                            }
                            restore_alt(terminal)?;
                        }
                        CmdResult::NeedGitDirPicker => {
                            drop_alt(terminal)?;
                            let mut picker = Picker::new(&app.cfg.git_dir, true);
                            if let Some(dir) = picker.run(terminal)? {
                                app.cfg.git_dir = dir.to_string_lossy().into_owned();
                                app.save();
                                app.popup = Some(format!("git dir → {}", app.cfg.git_dir));
                            }
                            restore_alt(terminal)?;
                        }
                        CmdResult::NeedBouncer => {
                            run_bouncer(terminal, app)?;
                        }
                        CmdResult::NeedCommitMsg { then_push } => {
                            drop_alt(terminal)?;
                            let msg = prompt_line("commit message: ")?;
                            restore_alt(terminal)?;
                            if !msg.trim().is_empty() {
                                commands::git_commit_and_maybe_push(app, msg.trim(), then_push);
                            } else {
                                app.popup = Some("commit cancelled (empty message)".into());
                            }
                        }
                        CmdResult::Ok => {
                            if app.show_tutorial {
                                run_tutorial(terminal, app)?;
                                app.show_tutorial = false;
                            }
                        }
                    }
                }
                KeyCode::Esc       => { cmd_buf.clear(); mode = Mode::Normal; }
                KeyCode::Backspace => { cmd_buf.pop(); }
                KeyCode::Char(c)   => cmd_buf.push(c),
                _ => {}
            },

            Mode::Search => match key.code {
                KeyCode::Enter => { mode = Mode::Normal; }
                KeyCode::Esc   => {
                    search_buf.clear();
                    app.rebuild_flat_view();
                    app.selected = 0;
                    mode = Mode::Normal;
                }
                KeyCode::Backspace => {
                    search_buf.pop();
                    app.rebuild_flat_view_filtered(&search_buf);
                    app.selected = 0;
                }
                KeyCode::Char(c) => {
                    search_buf.push(c);
                    app.rebuild_flat_view_filtered(&search_buf);
                    app.selected = 0;
                }
                _ => {}
            },

            Mode::Normal => match key.code {
                KeyCode::Char(':') => { cmd_buf.clear(); mode = Mode::Command; }
                KeyCode::Char('/') => { search_buf.clear(); mode = Mode::Search; }
                KeyCode::Char('q') => break,
                KeyCode::Char('j') | KeyCode::Down  => app.move_down(),
                KeyCode::Char('k') | KeyCode::Up    => app.move_up(),
                KeyCode::Char('p') => { app.cfg.preview_enabled = !app.cfg.preview_enabled; app.save(); }
                KeyCode::Char(' ') => app.toggle_group(),
                KeyCode::Enter => {
                    match app.selected_item().cloned() {
                        Some(Item::File { path, .. }) => {
                            drop_alt(terminal)?;
                            app.open_file(&path);
                            restore_alt(terminal)?;
                        }
                        Some(Item::Group(_)) => app.toggle_group(),
                        None => {}
                    }
                }
                _ => {}
            },
        }
    }
    Ok(())
}

fn draw_main<B: Backend>(
    f: &mut ratatui::Frame, app: &App, colors: &Colors,
    mode: &Mode, cmd_buf: &str, search_buf: &str, table_state: &mut TableState,
) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(2)])
        .split(area);
    draw_header(f, app, colors, chunks[0]);
    draw_body(f, app, colors, table_state, chunks[1]);
    draw_footer(f, app, colors, mode, cmd_buf, search_buf, chunks[2]);
}

fn draw_header(f: &mut ratatui::Frame, app: &App, colors: &Colors, area: Rect) {
    let last = app.cfg.last_opened.as_deref()
        .and_then(|p| Path::new(p).file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "none".into());
    let device_part = app.device_name.as_deref()
        .map(|d| format!("  [remote: {d}]"))
        .unwrap_or_default();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // row 0: just "confy" (+ remote tag if active)
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("confy", colors.group),
            Span::styled(device_part, colors.error),
        ])),
        chunks[0],
    );

    // row 1: previous: {x}    sort: name (asc)    config dir: /...
    let info = format!(
        "previous: {{{}}}    sort: {} ({})    config dir: {}",
        last, app.cfg.sort_mode, app.cfg.sort_order, app.cfg.config_dir
    );
    f.render_widget(
        Paragraph::new(Span::styled(info, colors.normal)),
        chunks[1],
    );
}

fn draw_body(f: &mut ratatui::Frame, app: &App, colors: &Colors, table_state: &mut TableState, area: Rect) {
    let preview_on = app.cfg.preview_enabled && area.width >= 60;
    let (list_area, preview_area) = if preview_on {
        let half = area.width / 2;
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(half), Constraint::Min(0)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    // build rows. groups span all columns, files use proper cells
    let rows: Vec<Row> = app.flat_view.iter().map(|item| match item {
        Item::Group(name) => {
            let collapsed = if app.cfg.collapsed_groups.contains(name) { "▶" } else { "▼" };
            let count = app.cfg.groups.get(name).map(|v| v.len()).unwrap_or(0);
            Row::new(vec![
                Cell::from(format!("{collapsed} {name}/ ({count} files)")),
                Cell::from(""),
                Cell::from(""),
            ]).style(colors.group)
        }
        Item::File { path, .. } => {
            // always show just the filename in the list, dir goes in the rightmost col
            let fname = Path::new(path).file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone());
            let exists = app.local_path(path).exists();
            let style  = if exists { colors.normal } else { colors.error };
            if app.cfg.settings.streamer_mode {
                Row::new(vec![
                    Cell::from(format!("  {fname}")),
                    Cell::from(""),
                    Cell::from("***"),
                ]).style(style)
            } else {
                let dir = app.display_dir(path);
                let (mtime, _size) = app.file_info(path);
                Row::new(vec![
                    Cell::from(format!("  {fname}")),
                    Cell::from(mtime),
                    Cell::from(dir),
                ]).style(style)
            }
        }
    }).collect();

    let table = Table::new(rows, [
        Constraint::Min(20),     // filename. takes all leftover space
        Constraint::Length(17),  // date    . fixed "2026-08-09 23:51"
        Constraint::Length(40),  // dir     . fixed, rightmost
    ])
    .block(Block::default().borders(Borders::TOP))
    .row_highlight_style(colors.highlight)
    .highlight_symbol("▶ ");
    f.render_stateful_widget(table, list_area, table_state);

    if let Some(parea) = preview_area {
        // Clear first. kills any floating characters left over from previous frames
        f.render_widget(Clear, parea);

        let (header, body) = match app.selected_item() {
            None => ("(nothing selected)".into(), vec![]),
            Some(Item::Group(name)) => {
                let count = app.cfg.groups.get(name).map(|v| v.len()).unwrap_or(0);
                (format!("{name}/"), vec![format!("{count} file(s) in group"), String::new(), "select a file to preview".into()])
            }
            Some(Item::File { path, .. }) => {
                let lines = app.preview_lines(path, (parea.height as usize).saturating_sub(3), (parea.width as usize).saturating_sub(2));
                let fname = Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                (fname, lines)
            }
        };
        let preview_text: Vec<Line> = std::iter::once(Line::from(Span::styled(header, colors.group)))
            .chain(std::iter::once(Line::from("")))
            .chain(body.into_iter().map(|l| Line::from(Span::styled(l, colors.normal))))
            .collect();
        f.render_widget(
            Paragraph::new(preview_text)
                .block(Block::default().borders(Borders::TOP | Borders::LEFT))
                .wrap(Wrap { trim: false }),
            parea,
        );
    }
}

fn draw_footer(f: &mut ratatui::Frame, app: &App, colors: &Colors, mode: &Mode, cmd_buf: &str, search_buf: &str, area: Rect) {
    // page = which screenful of items we're on, based on visible rows in the body
    // body height = terminal height minus header(2) + footer(2) rows, minus table border(1)
    let terminal_h = area.bottom() as usize; // area.bottom() is the y of the footer
    let body_h = terminal_h.saturating_sub(5).max(1); // 2 header + 2 footer + 1 border
    let total  = app.flat_view.len().max(1);
    let pages  = (total + body_h - 1) / body_h;
    let page   = (app.selected / body_h) + 1;

    let status = match mode {
        Mode::Command => format!("page {page}/{pages} ▌ :{cmd_buf}"),
        Mode::Search  => format!("page {page}/{pages} ▌ /{search_buf}"),
        Mode::Normal  => {
            let preview = if app.cfg.preview_enabled { "on" } else { "off" };
            format!("page {page}/{pages} ▌ p: preview {preview}")
        }
    };
    f.render_widget(
        Paragraph::new(Span::styled(status, colors.normal))
            .block(Block::default().borders(Borders::TOP)),
        area,
    );
}

fn draw_popup(f: &mut ratatui::Frame, msg: &str) {
    let area   = f.area();
    let width  = (msg.len() as u16 + 6).min(area.width.saturating_sub(4));
    let height = 3u16;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect { x, y, width, height };
    f.render_widget(Clear, popup_area);
    f.render_widget(
        Paragraph::new(Line::from(format!("  {msg}  ")))
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::REVERSED)),
        popup_area,
    );
}

fn run_tutorial<B: Backend>(terminal: &mut Terminal<B>, app: &App) -> Result<()> {
    let total = tutorial::STEPS.len();
    for (i, step) in tutorial::STEPS.iter().enumerate() {
        terminal.draw(|f| {
            let area   = f.area();
            let colors = Colors::from_app(app);
            let content_w = 64u16.min(area.width.saturating_sub(4));
            let content_h = (step.lines.len() as u16 + 4).min(area.height.saturating_sub(2));
            let x = area.x + (area.width.saturating_sub(content_w)) / 2;
            let y = area.y + (area.height.saturating_sub(content_h)) / 2;
            let popup_area = Rect { x, y, width: content_w, height: content_h };
            f.render_widget(Clear, popup_area);
            let lines: Vec<Line> = step.lines.iter()
                .map(|l| Line::from(Span::styled(*l, colors.normal))).collect();
            f.render_widget(
                Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL)
                        .title(format!(" {} ", step.title))
                        .title_alignment(Alignment::Center)),
                popup_area,
            );
            let counter = format!(" {}/{total} ", i + 1);
            let cx = popup_area.x + popup_area.width.saturating_sub(counter.len() as u16 + 1);
            let cy = popup_area.y + popup_area.height - 1;
            if cx < area.right() && cy < area.bottom() {
                f.render_widget(
                    Paragraph::new(Span::styled(counter, colors.group)),
                    Rect { x: cx, y: cy, width: popup_area.width / 2, height: 1 },
                );
            }
        })?;
        event::read()?;
    }
    Ok(())
}

fn drop_alt<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}

fn restore_alt<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::EnterAlternateScreen)?;
    // clear twice: once to flush any leftover editor output, once after ratatui takes over
    terminal.clear()?;
    terminal.draw(|f| f.render_widget(
        ratatui::widgets::Clear,
        f.area(),
    ))?;
    Ok(())
}

// ── bouncer (bulk blacklist menu) ─────────────────────────────────────────────

fn run_bouncer<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    use crate::app::Item;
    use crate::git;

    // build a flat list of (display_label, tracked_path_or_group, is_group)
    // seeded with current blacklist state
    let items: Vec<(String, String, bool)> = app.flat_view.iter().filter_map(|item| {
        match item {
            Item::Group(name) if name != "ungrouped" => {
                Some((format!("  [group] {name}/"), name.clone(), true))
            }
            Item::File { path, .. } => {
                let fname = std::path::Path::new(path)
                    .file_name().map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.clone());
                Some((format!("  {fname}"), path.clone(), false))
            }
            _ => None,
        }
    }).collect();

    // selected set: pre-populate with existing blacklist
    let mut selected: Vec<bool> = items.iter().map(|(_, key, is_group)| {
        if *is_group { git::is_group_blacklisted(app, key) }
        else         { git::is_blacklisted(app, key) }
    }).collect();

    let mut cursor = 0usize;
    let mut bouncer_state = ListState::default();

    loop {
        bouncer_state.select(Some(cursor));
        terminal.draw(|f| {
            let area = f.area();
            let colors = Colors::from_app(app);

            let list_items: Vec<ListItem> = items.iter().enumerate().map(|(i, (label, _, _))| {
                let marker = if selected[i] { "✖ " } else { "  " };
                let style = if selected[i] { colors.error } else { colors.normal };
                ListItem::new(Line::from(Span::styled(format!("{marker}{label}"), style)))
            }).collect();

            let list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL)
                    .title(" bouncer. git blacklist  space=toggle  Z=confirm  esc=cancel "))
                .highlight_style(colors.highlight)
                .highlight_symbol("▶ ");

            f.render_stateful_widget(list, area, &mut bouncer_state);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press { continue; }
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if cursor + 1 < items.len() { cursor += 1; }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if cursor > 0 { cursor -= 1; }
                }
                KeyCode::Char(' ') => {
                    selected[cursor] = !selected[cursor];
                }
                KeyCode::Char('Z') => {
                    // apply: set blacklists to match selected state
                    for (i, (_, key, is_group)) in items.iter().enumerate() {
                        if *is_group {
                            if selected[i] { git::blacklist_group(app, key); }
                            else           { git::unblacklist_group(app, key); }
                        } else {
                            if selected[i] { git::blacklist_file(app, key); }
                            else           { git::unblacklist_file(app, key); }
                        }
                    }
                    let count = selected.iter().filter(|&&s| s).count();
                    app.popup = Some(format!("blacklist updated. {count} item(s) excluded from :git sync"));
                    break;
                }
                KeyCode::Esc => break,
                _ => {}
            }
        }
    }
    Ok(())
}

// ── simple line prompt (used for commit message) ──────────────────────────────

/// drop back to the normal terminal and read a line from stdin.
/// simpler than trying to do inline text input in ratatui for a one-off prompt.
fn prompt_line(prompt: &str) -> Result<String> {
    use std::io::{BufRead, Write};
    print!("{prompt}");
    std::io::stdout().flush()?;
    let mut buf = String::new();
    std::io::stdin().lock().read_line(&mut buf)?;
    Ok(buf.trim_end_matches('\n').to_string())
}
