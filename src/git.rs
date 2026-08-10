use crate::app::App;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ── path resolution ───────────────────────────────────────────────────────────

/// work out where a tracked file should land in the dotfiles repo.
/// priority: group name > auto-sort by filename prefix.
/// ungrouped files with a shared prefix (e.g. hypr.conf + hyprlock.conf) get
/// a folder named after that prefix; truly unique files go in the root.
pub fn dest_path(git_dir: &Path, tracked: &str, group: &str, prefix_map: &HashMap<String, String>) -> PathBuf {
    let filename = Path::new(tracked)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".into());

    if group != "ungrouped" {
        // group name wins and goes into <git_dir>/<group>/<filename>
        git_dir.join(group).join(&filename)
    } else if let Some(prefix) = prefix_map.get(&filename) {
        // auto-sorted into a shared prefix folder
        git_dir.join(prefix).join(&filename)
    } else {
        // unique ungrouped file, goes in root
        git_dir.join(&filename)
    }
}

/// build a prefix map for ungrouped files: if 2+ filenames share a common
/// prefix (split on '.', '-', '_'), they get grouped under that prefix.
pub fn build_prefix_map(ungrouped: &[String]) -> HashMap<String, String> {
    let mut prefix_count: HashMap<String, usize> = HashMap::new();
    let mut file_prefix: HashMap<String, String> = HashMap::new();

    for tracked in ungrouped {
        let filename = Path::new(tracked)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        // extract prefix: everything before the first '.', '-', or '_'
        let prefix = filename
            .split(|c| c == '.' || c == '-' || c == '_')
            .next()
            .unwrap_or(&filename)
            .to_lowercase();
        if prefix.len() >= 3 {
            *prefix_count.entry(prefix.clone()).or_default() += 1;
            file_prefix.insert(filename, prefix);
        }
    }

    // only use prefix as a folder if 2+ files share it
    file_prefix.retain(|_, prefix| prefix_count.get(prefix).copied().unwrap_or(0) >= 2);
    file_prefix
}

// ── sync ─────────────────────────────────────────────────────────────────────

/// copy all non-blacklisted tracked files into the dotfiles repo, organised
/// by group (or auto-prefix for ungrouped). returns (copied, skipped) counts.
pub fn sync(app: &App) -> Result<(usize, usize)> {
    let git_dir = PathBuf::from(&app.cfg.git_dir);
    fs::create_dir_all(&git_dir)?;

    let ungrouped = app.cfg.groups.get("ungrouped").cloned().unwrap_or_default();
    let prefix_map = build_prefix_map(&ungrouped);

    let blacklist: std::collections::HashSet<&str> = app.cfg.git_blacklist.iter().map(|s| s.as_str()).collect();
    let blacklist_groups: std::collections::HashSet<&str> = app.cfg.git_blacklist_groups.iter().map(|s| s.as_str()).collect();

    let mut copied  = 0usize;
    let mut skipped = 0usize;

    for (group, files) in &app.cfg.groups {
        if blacklist_groups.contains(group.as_str()) {
            skipped += files.len();
            continue;
        }
        for tracked in files {
            if blacklist.contains(tracked.as_str()) {
                skipped += 1;
                continue;
            }
            let local = app.local_path(tracked);
            if !local.exists() {
                skipped += 1;
                continue;
            }
            let dest = dest_path(&git_dir, tracked, group, &prefix_map);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&local, &dest)
                .map_err(|e| anyhow!("failed to copy {} → {}: {e}", local.display(), dest.display()))?;
            copied += 1;
        }
    }
    Ok((copied, skipped))
}

// ── git helpers ───────────────────────────────────────────────────────────────

fn git_in(git_dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(git_dir)
        .output()
        .map_err(|_| anyhow!("git not found, is it installed?"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(anyhow!("{}", String::from_utf8_lossy(&out.stderr).trim()))
    }
}

pub fn is_git_repo(git_dir: &Path) -> bool {
    git_dir.join(".git").exists()
}

pub fn has_commits(git_dir: &Path) -> bool {
    git_in(git_dir, &["rev-parse", "HEAD"]).is_ok()
}

pub fn git_add_all(git_dir: &Path) -> Result<()> {
    git_in(git_dir, &["add", "."])?;
    Ok(())
}

pub fn git_commit(git_dir: &Path, message: &str) -> Result<()> {
    git_in(git_dir, &["commit", "-m", message])?;
    Ok(())
}

pub fn git_push(git_dir: &Path) -> Result<String> {
    // run push with inherited stdio so credential helpers work
    let status = Command::new("git")
        .args(["push"])
        .current_dir(git_dir)
        .status()
        .map_err(|_| anyhow!("git not found"))?;
    if status.success() {
        Ok("pushed!".into())
    } else {
        Err(anyhow!("git push failed! check your remote config"))
    }
}

pub fn git_has_staged(git_dir: &Path) -> bool {
    // exit 0 = clean/no staged changes, exit 1 = there are staged changes
    Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(git_dir)
        .status()
        .map(|s| !s.success())
        .unwrap_or(false)
}

// ── blacklist ops ─────────────────────────────────────────────────────────────

pub fn blacklist_file(app: &mut App, tracked: &str) {
    if !app.cfg.git_blacklist.contains(&tracked.to_string()) {
        app.cfg.git_blacklist.push(tracked.into());
        app.save();
    }
}

pub fn unblacklist_file(app: &mut App, tracked: &str) {
    app.cfg.git_blacklist.retain(|f| f != tracked);
    app.save();
}

pub fn blacklist_group(app: &mut App, group: &str) {
    if !app.cfg.git_blacklist_groups.contains(&group.to_string()) {
        app.cfg.git_blacklist_groups.push(group.into());
        app.save();
    }
}

pub fn unblacklist_group(app: &mut App, group: &str) {
    app.cfg.git_blacklist_groups.retain(|g| g != group);
    app.save();
}

pub fn is_blacklisted(app: &App, tracked: &str) -> bool {
    app.cfg.git_blacklist.iter().any(|f| f == tracked)
}

pub fn is_group_blacklisted(app: &App, group: &str) -> bool {
    app.cfg.git_blacklist_groups.iter().any(|g| g == group)
}
