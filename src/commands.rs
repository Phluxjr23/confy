use crate::app::{App, Item};
use crate::config::{builtin_themes, default_git_dir};
use crate::git;
use std::path::Path;

/// return value from handle_command
pub enum CmdResult {
    Ok,
    Quit,
    NeedPicker { group: String },
    NeedCdPicker,
    NeedGitDirPicker,
    NeedBouncer,
    NeedCommitMsg { then_push: bool },
}

pub fn handle(app: &mut App, raw: &str) -> CmdResult {
    let cmd   = raw.trim();
    let parts: Vec<&str> = cmd.splitn(2, char::is_whitespace).collect();
    let verb  = parts[0];
    let arg   = parts.get(1).map(|s| s.trim()).unwrap_or("");

    match (verb, arg) {
        ("q" | "quit", _) => return CmdResult::Quit,

        // ── file tracking ─────────────────────────────────────────────────────
        ("ac", "") => return CmdResult::NeedPicker { group: "ungrouped".into() },
        ("ac", g)  => return CmdResult::NeedPicker { group: g.to_string() },

        ("rm", _) => {
            if app.selected_file().is_some() {
                app.remove_selected();
            } else {
                app.popup = Some("select a file first".into());
            }
        }

        // ── last opened ───────────────────────────────────────────────────────
        ("l", _) => {
            if let Some(p) = app.cfg.last_opened.clone() {
                if app.local_path(&p).exists() {
                    app.open_file(&p);
                } else {
                    app.popup = Some("last file no longer exists".into());
                }
            } else {
                app.popup = Some("nothing opened yet".into());
            }
        }

        // ── rollback ──────────────────────────────────────────────────────────
        ("rb", _) => {
            if let Some(p) = app.selected_file().map(String::from) {
                match app.rollback(&p) {
                    Ok(msg) => app.popup = Some(msg),
                    Err(e)  => app.popup = Some(e.to_string()),
                }
            } else {
                app.popup = Some("select a file first".into());
            }
        }

        // ── elevated edit ─────────────────────────────────────────────────────
        ("su", _) => {
            if let Some(p) = app.selected_file().map(String::from) {
                match app.open_file_elevated(&p) {
                    Ok(_)  => {}
                    Err(e) => app.popup = Some(e.to_string()),
                }
            } else {
                app.popup = Some("select a file first".into());
            }
        }

        // ── groups ────────────────────────────────────────────────────────────
        ("ag", name) if !name.is_empty() => app.add_group(name),
        ("rg", name) if !name.is_empty() => app.remove_group(name),
        ("mg", name) if !name.is_empty() => app.move_to_group(name),

        // ── sort ──────────────────────────────────────────────────────────────
        ("sort", mode) if matches!(mode, "name" | "date" | "size") => {
            app.cfg.sort_mode = mode.to_string();
            app.save();
            app.rebuild_flat_view();
        }
        ("reverse", _) => {
            app.cfg.sort_order = if app.cfg.sort_order == "asc" { "desc".into() } else { "asc".into() };
            app.save();
            app.rebuild_flat_view();
        }

        // ── config dir ────────────────────────────────────────────────────────
        ("cd", "")      => return CmdResult::NeedCdPicker,
        ("cd", "reset") => {
            app.cfg.config_dir = dirs::home_dir()
                .map(|h| h.join(".config").to_string_lossy().into_owned())
                .unwrap_or_else(|| "/home".into());
            app.save();
            app.popup = Some(format!("config dir reset to {}", app.cfg.config_dir));
        }
        ("cd", path) => {
            let p = std::path::Path::new(path);
            if p.is_dir() {
                app.cfg.config_dir = path.to_string();
                app.save();
                app.popup = Some(format!("config dir → {path}"));
            } else {
                app.popup = Some(format!("not a directory: {path}"));
            }
        }

        // ── themes ────────────────────────────────────────────────────────────
        ("theme", "") => {
            let names = builtin_themes().keys().cloned().collect::<Vec<_>>().join(", ");
            app.popup = Some(format!("themes: {names}"));
        }
        ("theme", name) => {
            match app.set_theme(name) {
                Ok(_)  => app.popup = Some(format!("theme → {name}")),
                Err(e) => app.popup = Some(e.to_string()),
            }
        }

        // ── streamer mode hotswap ─────────────────────────────────────────────
        ("streamer", _) => {
            app.cfg.settings.streamer_mode = !app.cfg.settings.streamer_mode;
            let state = if app.cfg.settings.streamer_mode { "on" } else { "off" };
            app.save();
            app.popup = Some(format!("streamer mode {state}"));
        }

        // ── device ────────────────────────────────────────────────────────────
        ("device" | "ssh", _) if !app.cfg.settings.ssh_allow => {
            app.popup = Some("sshfs integration disabled (ssh_allow: false in config.json)".into());
        }
        ("device" | "ssh", "") => {
            if let Some(name) = &app.device_name {
                app.popup = Some(format!("currently on device: {name}"));
            } else {
                app.popup = Some("usage: :device <alias-or-user@host>  or  :device local".into());
            }
        }
        ("device" | "ssh", target) => {
            match app.switch_device(target) {
                Ok(_)  => {}
                Err(e) => app.popup = Some(e.to_string()),
            }
        }


        // ── git integration ───────────────────────────────────────────────────
        ("git", "sync") => {
            match git::sync(app) {
                Ok((copied, skipped)) => app.popup = Some(format!("synced! {copied} copied, {skipped} skipped")),
                Err(e) => app.popup = Some(format!("sync failed: {e}")),
            }
        }
        ("git", "commit") => {
            let git_dir = std::path::PathBuf::from(&app.cfg.git_dir);
            if !git::is_git_repo(&git_dir) {
                app.popup = Some(format!("{} is not a git repo", app.cfg.git_dir));
            } else if !git::has_commits(&git_dir) {
                app.popup = Some("no commits yet. make an initial commit manually first".into());
            } else {
                // sync first, then ui layer will prompt for commit message
                match git::sync(app) {
                    Err(e) => app.popup = Some(format!("sync failed: {e}")),
                    Ok(_)  => return CmdResult::NeedCommitMsg { then_push: false },
                }
            }
        }
        ("git", "push") => {
            let git_dir = std::path::PathBuf::from(&app.cfg.git_dir);
            if !git::is_git_repo(&git_dir) {
                app.popup = Some(format!("{} is not a git repo", app.cfg.git_dir));
            } else if !git::has_commits(&git_dir) {
                app.popup = Some("no commits yet. make an initial commit manually first".into());
            } else {
                match git::sync(app) {
                    Err(e) => app.popup = Some(format!("sync failed: {e}")),
                    Ok(_)  => return CmdResult::NeedCommitMsg { then_push: true },
                }
            }
        }
        ("git", s) if s.starts_with("dir") => {
            let path_arg = s.trim_start_matches("dir").trim();
            if path_arg.is_empty() {
                return CmdResult::NeedGitDirPicker;
            } else {
                let p = std::path::Path::new(path_arg);
                if p.is_dir() || p.parent().map(|pp| pp.exists()).unwrap_or(false) {
                    app.cfg.git_dir = path_arg.to_string();
                    app.save();
                    app.popup = Some(format!("git dir → {path_arg}"));
                } else {
                    app.popup = Some(format!("invalid path: {path_arg}"));
                }
            }
        }
        ("git", "reset") => {
            app.cfg.git_dir = default_git_dir();
            app.save();
            app.popup = Some(format!("git dir reset to {}", app.cfg.git_dir));
        }
        ("git", _) => {
            app.popup = Some("usage: :git sync | :git commit | :git push | :git dir <path> | :git reset".into());
        }

        // ── blacklist ─────────────────────────────────────────────────────────
        ("bl", "") => {
            match app.selected_item() {
                Some(Item::File { path, group }) => {
                    let (p, g) = (path.clone(), group.clone());
                    if git::is_blacklisted(app, &p) {
                        git::unblacklist_file(app, &p);
                        app.popup = Some(format!("removed {} from git blacklist", Path::new(&p).file_name().unwrap_or_default().to_string_lossy()));
                    } else {
                        git::blacklist_file(app, &p);
                        app.popup = Some(format!("blacklisted {}", Path::new(&p).file_name().unwrap_or_default().to_string_lossy()));
                    }
                }
                Some(Item::Group(name)) => {
                    let g = name.clone();
                    if git::is_group_blacklisted(app, &g) {
                        git::unblacklist_group(app, &g);
                        app.popup = Some(format!("removed group {g} from git blacklist"));
                    } else {
                        git::blacklist_group(app, &g);
                        app.popup = Some(format!("blacklisted group {g}"));
                    }
                }
                None => app.popup = Some("select a file or group first".into()),
            }
        }
        ("bouncer", _) => return CmdResult::NeedBouncer,

        // ── help ──────────────────────────────────────────────────────────────
        ("h" | "help", _) => {
            app.show_tutorial = true;
        }

        // ── unknown ───────────────────────────────────────────────────────────
        _ => {
            app.popup = Some(format!("unknown command: {cmd}"));
        }
    }

    CmdResult::Ok
}

// separate pub fn so ui.rs can call it after prompting for commit message
pub fn git_commit_and_maybe_push(app: &mut App, msg: &str, push: bool) {
    let git_dir = std::path::PathBuf::from(&app.cfg.git_dir);
    if !git::is_git_repo(&git_dir) {
        app.popup = Some(format!("{} is not a git repo. run `git init` there first", app.cfg.git_dir));
        return;
    }
    if !git::has_commits(&git_dir) {
        app.popup = Some("no commits yet. make an initial commit manually first".into());
        return;
    }
    match git::git_add_all(&git_dir) {
        Err(e) => { app.popup = Some(format!("git add failed: {e}")); return; }
        Ok(_)  => {}
    }
    if !git::git_has_staged(&git_dir) {
        app.popup = Some("nothing to commit. files are unchanged since last commit".into());
        return;
    }
    match git::git_commit(&git_dir, msg) {
        Err(e) => { app.popup = Some(format!("git commit failed: {e}")); return; }
        Ok(_)  => {}
    }
    if push {
        match git::git_push(&git_dir) {
            Err(e) => app.popup = Some(format!("committed, but push failed: {e}")),
            Ok(_)  => app.popup = Some("committed and pushed!".into()),
        }
    } else {
        app.popup = Some("committed!".into());
    }
}
