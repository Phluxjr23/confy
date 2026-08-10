use crate::config::{self, AppConfig, Settings, builtin_themes, confy_config_dir, confy_config_file};
use crate::device::{self, MountInfo, RemoteFormat};
use anyhow::Result;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

// ── flat view item ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Item {
    Group(String),
    File { path: String, group: String },
}

// ── app state ─────────────────────────────────────────────────────────────────

pub struct App {
    pub cfg: AppConfig,

    // active config location (repointed when a device is mounted)
    pub active_config_dir:  PathBuf,
    pub active_config_file: PathBuf,

    // device mode
    pub mount:       Option<MountInfo>,
    pub device_name: Option<String>,
    local_state:     Option<Box<AppConfig>>,

    // view state
    pub flat_view:  Vec<Item>,
    pub selected:   usize,
    pub scroll:     usize,

    // ui state
    pub popup:        Option<String>,
    pub show_tutorial: bool,
}

impl App {
    pub fn new() -> Self {
        let config_dir  = confy_config_dir();
        let config_file = confy_config_file();

        config::migrate_if_needed(&config_dir);

        let (cfg, first) = config::load(&config_file).unwrap_or_else(|e| {
            eprintln!("warn: {e}");
            (AppConfig::default(), true)
        });

        let mut app = Self {
            cfg,
            active_config_dir:  config_dir,
            active_config_file: config_file,
            mount:       None,
            device_name: None,
            local_state: None,
            flat_view:   vec![],
            selected:    0,
            scroll:      0,
            popup:       None,
            show_tutorial: first,
        };
        app.rebuild_flat_view();
        app
    }

    // ── persistence ───────────────────────────────────────────────────────────

    pub fn save(&self) {
        let _ = config::save(&self.active_config_file, &self.cfg);
    }

    // ── path resolution ───────────────────────────────────────────────────────

    /// tracked path → local filesystem path (transparent on local; rewritten on device)
    pub fn local_path(&self, tracked: &str) -> PathBuf {
        match &self.mount {
            Some(m) => m.to_local_path(tracked),
            None    => PathBuf::from(tracked),
        }
    }

    /// local filesystem path → tracked path (reverse of local_path)
    pub fn tracked_path(&self, local: &Path) -> String {
        match &self.mount {
            Some(m) => m.to_remote_path(local),
            None    => local.to_string_lossy().into_owned(),
        }
    }

    // ── flat view ─────────────────────────────────────────────────────────────

    pub fn rebuild_flat_view(&mut self) {
        self.flat_view.clear();
        let mut group_names: Vec<_> = self.cfg.groups.keys().cloned().collect();
        // ungrouped always goes last regardless of sort mode
        group_names.sort_by(|a, b| match (a.as_str(), b.as_str()) {
            ("ungrouped", _) => std::cmp::Ordering::Greater,
            (_, "ungrouped") => std::cmp::Ordering::Less,
            _ => a.cmp(b),
        });
        for group_name in group_names {
            self.flat_view.push(Item::Group(group_name.clone()));
            if !self.cfg.collapsed_groups.contains(&group_name) {
                let mut files = self.cfg.groups[&group_name].clone();
                self.sort_files(&mut files);
                for f in files {
                    self.flat_view.push(Item::File { path: f, group: group_name.clone() });
                }
            }
        }
    }

    pub fn rebuild_flat_view_filtered(&mut self, query: &str) {
        self.flat_view.clear();
        let q = query.to_lowercase();
        let mut group_names: Vec<_> = self.cfg.groups.keys().cloned().collect();
        group_names.sort_by(|a, b| match (a.as_str(), b.as_str()) {
            ("ungrouped", _) => std::cmp::Ordering::Greater,
            (_, "ungrouped") => std::cmp::Ordering::Less,
            _ => a.cmp(b),
        });
        for group_name in group_names {
            let files: Vec<_> = self.cfg.groups[&group_name]
                .iter()
                .filter(|f| {
                    let name = Path::new(f).file_name()
                        .map(|n| n.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    name.contains(&q) || group_name.to_lowercase().contains(&q)
                })
                .cloned()
                .collect();

            if !files.is_empty() || group_name.to_lowercase().contains(&q) {
                self.flat_view.push(Item::Group(group_name.clone()));
                if !self.cfg.collapsed_groups.contains(&group_name) {
                    let mut sorted = files;
                    self.sort_files(&mut sorted);
                    for f in sorted {
                        self.flat_view.push(Item::File { path: f, group: group_name.clone() });
                    }
                }
            }
        }
    }

    fn sort_files(&self, files: &mut Vec<String>) {
        match self.cfg.sort_mode.as_str() {
            "date" => files.sort_by_key(|f| {
                self.local_path(f).metadata()
                    .and_then(|m| m.modified())
                    .map(|t| t.elapsed().unwrap_or_default())
                    .unwrap_or_default()
            }),
            "size" => files.sort_by_key(|f| {
                self.local_path(f).metadata().map(|m| m.len()).unwrap_or(0)
            }),
            _ => files.sort_by(|a, b| {
                let an = Path::new(a).file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                let bn = Path::new(b).file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
                an.cmp(&bn)
            }),
        }
        if self.cfg.sort_order == "desc" {
            files.reverse();
        }
    }

    // ── selected item helpers ─────────────────────────────────────────────────

    pub fn selected_item(&self) -> Option<&Item> {
        self.flat_view.get(self.selected)
    }

    pub fn selected_file(&self) -> Option<&str> {
        match self.selected_item()? {
            Item::File { path, .. } => Some(path),
            _ => None,
        }
    }

    // ── navigation ────────────────────────────────────────────────────────────

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll {
                self.scroll = self.selected;
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.flat_view.len() {
            self.selected += 1;
        }
    }

    pub fn clamp_selected(&mut self) {
        if self.flat_view.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.flat_view.len() {
            self.selected = self.flat_view.len() - 1;
        }
    }

    // ── group ops ─────────────────────────────────────────────────────────────

    pub fn toggle_group(&mut self) {
        if let Some(Item::Group(name)) = self.selected_item().cloned() {
            if self.cfg.collapsed_groups.contains(&name) {
                self.cfg.collapsed_groups.remove(&name);
            } else {
                self.cfg.collapsed_groups.insert(name);
            }
            self.save();
            self.rebuild_flat_view();
        }
    }

    pub fn add_group(&mut self, name: &str) {
        if !name.is_empty() && !self.cfg.groups.contains_key(name) {
            self.cfg.groups.insert(name.into(), vec![]);
            self.save();
            self.rebuild_flat_view();
        }
    }

    pub fn remove_group(&mut self, name: &str) {
        if name == "ungrouped" { return; }
        if let Some(files) = self.cfg.groups.remove(name) {
            self.cfg.groups.entry("ungrouped".into()).or_default().extend(files);
            self.save();
            self.rebuild_flat_view();
        }
    }

    pub fn move_to_group(&mut self, group_name: &str) {
        let Some(Item::File { path, group: old_group }) = self.selected_item().cloned() else { return; };
        if !self.cfg.groups.contains_key(group_name) {
            self.cfg.groups.insert(group_name.into(), vec![]);
        }
        if let Some(files) = self.cfg.groups.get_mut(&old_group) {
            files.retain(|f| f != &path);
        }
        self.cfg.groups.entry(group_name.into()).or_default().push(path);
        self.save();
        self.rebuild_flat_view();
        self.clamp_selected();
    }

    // ── file tracking ops ─────────────────────────────────────────────────────

    pub fn track_file(&mut self, local_picked: &Path, group: &str) -> bool {
        let tracked = self.tracked_path(local_picked);
        if self.cfg.groups.values().any(|v| v.contains(&tracked)) {
            self.popup = Some("already tracked!".into());
            return false;
        }
        self.cfg.groups.entry(group.into()).or_default().push(tracked);
        self.save();
        self.rebuild_flat_view();
        true
    }

    pub fn remove_selected(&mut self) {
        let Some(Item::File { path, group }) = self.selected_item().cloned() else { return; };
        if let Some(files) = self.cfg.groups.get_mut(&group) {
            files.retain(|f| f != &path);
        }
        self.save();
        self.rebuild_flat_view();
        self.clamp_selected();
    }

    // ── rollback ──────────────────────────────────────────────────────────────

    pub fn save_backup(&self, tracked: &str) {
        if !self.cfg.settings.rollback { return; }
        let local = self.local_path(tracked);
        let bak_name = format!("{}.confbak", Path::new(tracked).file_name().unwrap_or_default().to_string_lossy());
        let bak_path = PathBuf::from("/tmp").join(bak_name);
        let Ok(data) = fs::read(&local) else { return; };
        let Ok(file) = fs::File::create(&bak_path) else { return; };
        let mut gz = GzEncoder::new(file, Compression::default());
        let _ = gz.write_all(&data);
    }

    pub fn rollback(&self, tracked: &str) -> Result<String> {
        let local = self.local_path(tracked);
        let bak_name = format!("{}.confbak", Path::new(tracked).file_name().unwrap_or_default().to_string_lossy());
        let bak_path = PathBuf::from("/tmp").join(bak_name);
        if !bak_path.exists() {
            return Err(anyhow::anyhow!("no backup found in /tmp"));
        }
        let file = fs::File::open(&bak_path)?;
        let mut gz = GzDecoder::new(file);
        let mut buf = vec![];
        gz.read_to_end(&mut buf)?;
        fs::write(&local, &buf)?;
        Ok(format!("rolled back {}!", Path::new(tracked).file_name().unwrap_or_default().to_string_lossy()))
    }

    // ── open file ─────────────────────────────────────────────────────────────

    /// resolve the editor to use: config.json > $EDITOR > nano
    pub fn editor(&self) -> String {
        self.cfg.settings.editor.clone()
            .filter(|e| !e.is_empty())
            .or_else(|| std::env::var("EDITOR").ok())
            .unwrap_or_else(|| "nano".into())
    }

    pub fn open_file(&mut self, tracked: &str) {
        self.save_backup(tracked);
        let local = self.local_path(tracked);
        let editor = self.editor();
        let _ = std::process::Command::new(&editor).arg(&local).status();
        self.cfg.last_opened = Some(tracked.into());
        self.save();
    }

    pub fn open_file_elevated(&mut self, tracked: &str) -> Result<()> {
        if !device::pkexec_available() {
            return Err(anyhow::anyhow!("pkexec not found. needs polkit"));
        }
        if let Some(m) = &self.mount {
            if !m.su_available {
                return Err(anyhow::anyhow!(
                    "can't :su through this mount. add 'user_allow_other' to /etc/fuse.conf and reconnect"
                ));
            }
        }
        self.save_backup(tracked);
        let local = self.local_path(tracked);
        let editor = self.editor();
        let editor_path = which_editor(&editor)
            .ok_or_else(|| anyhow::anyhow!("editor '{editor}' not found on PATH"))?;
        let status = std::process::Command::new("pkexec")
            .args([&editor_path, &local.to_string_lossy().into_owned()])
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("pkexec exited with code {} (auth cancelled?)", status.code().unwrap_or(-1)));
        }
        self.cfg.last_opened = Some(tracked.into());
        self.save();
        Ok(())
    }

    // ── themes ────────────────────────────────────────────────────────────────

    pub fn set_theme(&mut self, name: &str) -> Result<()> {
        let themes = builtin_themes();
        let colors = themes.get(name).ok_or_else(|| {
            let names = themes.keys().cloned().collect::<Vec<_>>().join(", ");
            anyhow::anyhow!("unknown theme '{name}'. options: {names}")
        })?;
        self.cfg.settings.theme   = name.into();
        self.cfg.settings.colors  = colors.clone();
        self.save();
        Ok(())
    }

    pub fn theme_names() -> Vec<&'static str> {
        builtin_themes().keys().copied().collect()
    }

    // ── device mode ───────────────────────────────────────────────────────────

    pub fn switch_device(&mut self, arg: &str) -> Result<()> {
        let arg = arg.trim();
        if arg.is_empty() || arg.eq_ignore_ascii_case("local") {
            self.switch_to_local();
            return Ok(());
        }

        if !device::sshfs_available() {
            return Err(anyhow::anyhow!("sshfs not found. install it: {}", device::SSHFS_URL));
        }

        let target = device::resolve_target(arg);

        // unmount existing device if any
        if let Some(m) = &self.mount {
            device::unmount(&m.mountpoint);
        }

        let mount_root = config::mount_root();
        let info = device::mount(&target, &mount_root)?;

        let confy_dir = info.to_local_path(&format!("{}/.config/confy", info.remote_home));
        let fmt = device::check_remote_format(&confy_dir);

        if fmt == RemoteFormat::Unreachable {
            device::unmount(&info.mountpoint);
            return Err(anyhow::anyhow!("couldn't read remote confy dir on {target}"));
        }
        if fmt == RemoteFormat::Legacy {
            device::unmount(&info.mountpoint);
            return Err(anyhow::anyhow!(
                "{target} is running an older confy (tracked.json). update confy there first."
            ));
        }

        // stash local state
        if self.local_state.is_none() {
            self.local_state = Some(Box::new(self.cfg.clone()));
        }

        let su_ok = info.su_available;
        let note  = if fmt == RemoteFormat::Empty { " (empty, nothing tracked there yet)" } else { "" };

        self.device_name = Some(arg.into());
        self.active_config_dir  = confy_dir.clone();
        self.active_config_file = confy_dir.join("config.json");
        self.cfg.config_dir     = format!("{}/.config", info.remote_home);
        self.mount = Some(info);

        config::migrate_if_needed(&self.active_config_dir);
        let (remote_cfg, _) = config::load(&self.active_config_file)
            .unwrap_or_else(|_| (AppConfig::default(), true));
        self.cfg = remote_cfg;

        self.selected = 0;
        self.scroll   = 0;
        self.rebuild_flat_view();

        if !su_ok {
            self.popup = Some(format!(
                "device → {}{note}. note: :su won't work. add 'user_allow_other' to /etc/fuse.conf",
                self.device_name.as_deref().unwrap_or("?")
            ));
        } else {
            self.popup = Some(format!("device → {}{note}", self.device_name.as_deref().unwrap_or("?")));
        }

        Ok(())
    }

    pub fn switch_to_local(&mut self) {
        if let Some(m) = self.mount.take() {
            device::unmount(&m.mountpoint);
        }
        self.device_name = None;

        if let Some(saved) = self.local_state.take() {
            self.cfg = *saved;
        }

        self.active_config_dir  = config::confy_config_dir();
        self.active_config_file = config::confy_config_file();
        self.selected = 0;
        self.scroll   = 0;
        self.rebuild_flat_view();
        self.popup = Some("device → local".into());
    }

    // ── display path (streamer mode aware) ───────────────────────────────────

    /// returns the path as it should be shown in the ui.
    /// in streamer mode, only the filename is shown. no /home/username leaking on stream.
    pub fn display_path<'a>(&self, tracked: &'a str) -> std::borrow::Cow<'a, str> {
        if self.cfg.settings.streamer_mode {
            Path::new(tracked)
                .file_name()
                .map(|n| std::borrow::Cow::Owned(n.to_string_lossy().into_owned()))
                .unwrap_or(std::borrow::Cow::Borrowed(tracked))
        } else {
            std::borrow::Cow::Borrowed(tracked)
        }
    }

    /// parent dir for display. hidden in streamer mode
    pub fn display_dir(&self, tracked: &str) -> String {
        if self.cfg.settings.streamer_mode {
            return "***".into();
        }
        Path::new(tracked)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    // ── file info ─────────────────────────────────────────────────────────────

    pub fn file_info(&self, tracked: &str) -> (String, String) {
        let local = self.local_path(tracked);
        match local.metadata() {
            Ok(m) => {
                let mtime = m.modified().ok()
                    .and_then(|t| {
                        use std::time::UNIX_EPOCH;
                        let secs = t.duration_since(UNIX_EPOCH).ok()?.as_secs();
                        // basic formatting without chrono dep
                        Some(format_unix_ts(secs))
                    })
                    .unwrap_or_else(|| "unknown".into());
                let size = format_size(m.len());
                (mtime, size)
            }
            Err(_) => ("unknown".into(), "unknown".into()),
        }
    }

    pub fn preview_lines(&self, tracked: &str, max_lines: usize, max_width: usize) -> Vec<String> {
        let local = self.local_path(tracked);
        if !local.exists()  { return vec!["(file not found)".into()]; }
        if local.is_dir()   { return vec!["(directory)".into()]; }
        match local.metadata() {
            Ok(m) if m.len() > 5 * 1024 * 1024 => return vec!["(file too large to preview, 5MB+)".into()],
            _ => {}
        }
        match fs::read_to_string(&local) {
            Err(_) => vec!["(binary or unreadable file)".into()],
            Ok(content) => {
                let mut lines: Vec<String> = content
                    .lines()
                    .take(max_lines)
                    .map(|l| {
                        let char_count = l.chars().count();
                        if char_count > max_width {
                            // slice on char boundaries, never bytes. avoids panic on unicode
                            let truncated: String = l.chars().take(max_width.saturating_sub(1)).collect();
                            format!("{truncated}…")
                        } else {
                            l.to_string()
                        }
                    })
                    .collect();
                if content.lines().count() > max_lines { lines.push("...".into()); }
                if lines.is_empty() { lines.push("(empty file)".into()); }
                lines
            }
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn format_size(mut size: u64) -> String {
    for unit in ["B", "KB", "MB", "GB"] {
        if size < 1024 { return format!("{size}{unit}"); }
        size /= 1024;
    }
    format!("{size}TB")
}

fn format_unix_ts(secs: u64) -> String {
    // basic yyyy-mm-dd hh:mm without chrono
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    let mut y = 1970u64;
    let mut d = days;
    loop {
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let yd = if leap { 366 } else { 365 };
        if d < yd { break; }
        d -= yd; y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let months = [31u64,if leap {29} else {28},31,30,31,30,31,31,30,31,30,31];
    let mut mo = 1u64;
    for mlen in months {
        if d < mlen { break; }
        d -= mlen; mo += 1;
    }
    format!("{y:04}-{mo:02}-{:02} {h:02}:{m:02}", d + 1)
}

fn which_editor(name: &str) -> Option<String> {
    std::env::var_os("PATH").and_then(|p| {
        std::env::split_paths(&p)
            .map(|d| d.join(name))
            .find(|p| p.is_file())
            .map(|p| p.to_string_lossy().into_owned())
    })
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(m) = &self.mount {
            device::unmount(&m.mountpoint);
        }
    }
}
