use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::path::{Path, PathBuf};

pub struct Picker {
    pub cwd:      PathBuf,
    pub entries:  Vec<PathBuf>,
    pub state:    ListState,
    pub pick_dir: bool, // true = dir-select mode (:cd), false = file-select mode (:ac)
}

impl Picker {
    pub fn new(start: &str, pick_dir: bool) -> Self {
        let cwd = PathBuf::from(start).canonicalize().unwrap_or_else(|_| PathBuf::from(start));
        let mut p = Self { cwd, entries: vec![], state: ListState::default(), pick_dir };
        p.load();
        p
    }

    fn load(&mut self) {
        let parent = self.cwd.parent().map(|p| p.to_path_buf());
        let mut entries: Vec<PathBuf> = match std::fs::read_dir(&self.cwd) {
            Ok(rd) => {
                let mut v: Vec<_> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
                v.sort_by(|a, b| {
                    b.is_dir().cmp(&a.is_dir())
                        .then(a.file_name().unwrap_or_default().to_string_lossy().to_lowercase()
                            .cmp(&b.file_name().unwrap_or_default().to_string_lossy().to_lowercase()))
                });
                v
            }
            Err(_) => vec![],
        };
        if let Some(p) = parent {
            entries.insert(0, p); // ".." entry
        }
        self.entries = entries;
        self.state.select(Some(0));
    }

    fn selected_path(&self) -> Option<&PathBuf> {
        self.state.selected().and_then(|i| self.entries.get(i))
    }

    fn is_parent_entry(&self, path: &Path) -> bool {
        Some(path) == self.cwd.parent().map(|p| p as &Path)
    }

    /// run the picker, returning the chosen path or None if cancelled
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<Option<PathBuf>> {
        loop {
            terminal.draw(|f| self.render(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press { continue; }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        if self.pick_dir {
                            return Ok(Some(self.cwd.clone()));
                        }
                        return Ok(None);
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let i = self.state.selected().unwrap_or(0);
                        if i + 1 < self.entries.len() {
                            self.state.select(Some(i + 1));
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        let i = self.state.selected().unwrap_or(0);
                        if i > 0 {
                            self.state.select(Some(i - 1));
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(path) = self.selected_path().cloned() {
                            if self.is_parent_entry(&path) || path.is_dir() {
                                self.cwd = path.canonicalize().unwrap_or(path);
                                self.load();
                            } else if self.pick_dir {
                                return Ok(Some(self.cwd.clone()));
                            } else {
                                return Ok(Some(path));
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        if let Some(parent) = self.cwd.parent().map(|p| p.to_path_buf()) {
                            self.cwd = parent;
                            self.load();
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn render(&mut self, f: &mut ratatui::Frame) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        let mode = if self.pick_dir { "select dir  q/esc=use this dir" } else { "select file  q/esc=cancel" };
        let header = Paragraph::new(format!("  {}  |  {}", self.cwd.display(), mode))
            .block(Block::default().borders(Borders::ALL).title(" confy file picker "));
        f.render_widget(header, chunks[0]);

        let items: Vec<ListItem> = self.entries.iter().map(|p| {
            let is_up = self.is_parent_entry(p);
            let name = if is_up {
                "../".to_string()
            } else if p.is_dir() {
                format!("{}/", p.file_name().unwrap_or_default().to_string_lossy())
            } else {
                p.file_name().unwrap_or_default().to_string_lossy().into_owned()
            };
            let style = if p.is_dir() || is_up {
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(format!("  {name}"), style)))
        }).collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, chunks[1], &mut self.state);
    }
}
