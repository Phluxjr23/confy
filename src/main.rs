// confy - a config manager for linux/unix systems
// Copyright (C) 2025-2026 phluxjr
// Licensed under GPL-3.0-or-later

mod app;
mod commands;
mod config;
mod device;
mod git;
mod picker;
mod tutorial;
mod ui;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

fn main() -> Result<()> {
    // restore terminal on panic so the user isn't left with a mangled shell
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(io::stderr(), LeaveAlternateScreen);
        orig_hook(info);
    }));

    let mut app = app::App::new();

    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    let result = ui::run(&mut terminal, &mut app);

    // always clean up terminal even on error
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
