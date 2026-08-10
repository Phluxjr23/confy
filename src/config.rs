use anyhow::{Context, Result};
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

// ── paths ─────────────────────────────────────────────────────────────────────

pub fn confy_config_dir() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".config")
        .join("confy")
}

pub fn confy_config_file() -> PathBuf {
    confy_config_dir().join("config.json")
}

pub fn confy_legacy_file() -> PathBuf {
    confy_config_dir().join("tracked.json")
}

pub fn mount_root() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".cache")
        .join("confy")
        .join("mounts")
}

// ── themes ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorSettings {
    pub bg: String,
    pub fg: String,
    pub highlight: String,
    pub group: String,
    pub border: String,
}

impl Default for ColorSettings {
    fn default() -> Self {
        Self {
            bg: "default".into(),
            fg: "default".into(),
            highlight: "#cba6f7".into(),
            group: "#89b4fa".into(),
            border: "default".into(),
        }
    }
}

pub fn builtin_themes() -> HashMap<&'static str, ColorSettings> {
    let mut m = HashMap::new();
    m.insert("catppuccin", ColorSettings {
        highlight: "#cba6f7".into(),
        group:     "#89b4fa".into(),
        ..Default::default()
    });
    m.insert("dracula", ColorSettings {
        highlight: "#bd93f9".into(),
        group:     "#8be9fd".into(),
        ..Default::default()
    });
    m.insert("gruvbox", ColorSettings {
        highlight: "#fabd2f".into(),
        group:     "#83a598".into(),
        ..Default::default()
    });
    m.insert("nord", ColorSettings {
        highlight: "#88c0d0".into(),
        group:     "#81a1c1".into(),
        ..Default::default()
    });
    m.insert("tokyo-night", ColorSettings {
        highlight: "#bb9af7".into(),
        group:     "#7aa2f7".into(),
        ..Default::default()
    });
    m.insert("one-dark", ColorSettings {
        highlight: "#c678dd".into(),
        group:     "#61afef".into(),
        ..Default::default()
    });
    m
}

// ── settings ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_true")]
    pub rollback: bool,
    #[serde(default = "default_true")]
    pub first_startup: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub colors: ColorSettings,

    // background: draw a solid bg instead of terminal default (textual-style)
    #[serde(default)]
    pub background_enable: bool,
    #[serde(default = "default_bg_color")]
    pub background_color: String,

    // streamer mode: hide /home/username paths, show only filenames
    #[serde(default)]
    pub streamer_mode: bool,
    // in streamer mode, also hide files that are git-blacklisted
    #[serde(default = "default_true")]
    pub streamer_hide_git: bool,

    // override $EDITOR (useful for .desktop launchers)
    #[serde(default)]
    pub editor: Option<String>,

    // allow sshfs / :device integration (can be disabled if sshfs isn't installed)
    #[serde(default = "default_true")]
    pub ssh_allow: bool,
}

fn default_true()     -> bool   { true }
fn default_theme()    -> String { "catppuccin".into() }
fn default_bg_color() -> String { "#1e1e2e".into() }

impl Default for Settings {
    fn default() -> Self {
        Self {
            rollback:           true,
            first_startup:      true,
            theme:              "catppuccin".into(),
            colors:             ColorSettings::default(),
            background_enable:  false,
            background_color:   default_bg_color(),
            streamer_mode:      false,
            streamer_hide_git:  true,
            editor:             None,
            ssh_allow:          true,
        }
    }
}

// ── main config ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub groups: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub last_opened: Option<String>,
    #[serde(default)]
    pub collapsed_groups: HashSet<String>,
    #[serde(default = "default_sort_mode")]
    pub sort_mode: String,
    #[serde(default = "default_sort_order")]
    pub sort_order: String,
    #[serde(default = "default_config_dir")]
    pub config_dir: String,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub preview_enabled: bool,

    // git integration
    #[serde(default = "default_git_dir")]
    pub git_dir: String,
    #[serde(default)]
    pub git_blacklist: Vec<String>,
    #[serde(default)]
    pub git_blacklist_groups: Vec<String>,
    #[serde(default)]
    pub git_auto_push: bool,
    #[serde(default)]
    pub git_auto_commit: bool,
}

fn default_sort_mode()  -> String { "name".into() }
fn default_sort_order() -> String { "asc".into() }
fn default_config_dir() -> String {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("/home"))
        .join(".config")
        .to_string_lossy()
        .into_owned()
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut groups = HashMap::new();
        groups.insert("ungrouped".into(), vec![]);
        Self {
            groups,
            last_opened: None,
            collapsed_groups: HashSet::new(),
            sort_mode: default_sort_mode(),
            sort_order: default_sort_order(),
            config_dir: default_config_dir(),
            settings: Settings::default(),
            preview_enabled: false,
            git_dir: default_git_dir(),
            git_blacklist: vec![],
            git_blacklist_groups: vec![],
            git_auto_push: false,
            git_auto_commit: false,
        }
    }
}

// ── legacy migration ──────────────────────────────────────────────────────────

/// if tracked.json exists and config.json doesn't, copy it over
pub fn migrate_if_needed(config_dir: &Path) -> Option<String> {
    let legacy = config_dir.join("tracked.json");
    let current = config_dir.join("config.json");
    if !current.exists() && legacy.exists() {
        if fs::copy(&legacy, &current).is_ok() {
            return Some("migrated tracked.json → config.json!".into());
        }
    }
    None
}

// ── load / save ───────────────────────────────────────────────────────────────

pub fn load(path: &Path) -> Result<(AppConfig, bool)> {
    if !path.exists() {
        return Ok((AppConfig::default(), true)); // first startup
    }
    let raw = fs::read_to_string(path)
        .with_context(|| format!("couldn't read {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok((AppConfig::default(), true));
    }
    // handle legacy format: {"files": [...]} instead of {"groups": {...}}
    let mut val: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("couldn't parse {}", path.display()))?;
    if let Some(files) = val.get("files").cloned() {
        let mut groups = serde_json::Map::new();
        groups.insert("ungrouped".into(), files);
        val["groups"] = serde_json::Value::Object(groups);
    }
    let cfg: AppConfig = serde_json::from_value(val)
        .with_context(|| format!("couldn't deserialise {}", path.display()))?;
    let first = cfg.settings.first_startup;
    Ok((cfg, first))
}

pub fn save(path: &Path, cfg: &AppConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(cfg)?;
    fs::write(path, json)?;
    Ok(())
}

// ── git helpers (used by git.rs) ──────────────────────────────────────────────

pub fn default_git_dir() -> String {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("/home"))
        .join("dotfiles-git")
        .to_string_lossy()
        .into_owned()
}
