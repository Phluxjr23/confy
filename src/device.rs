use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub const SSHFS_URL: &str = "https://github.com/libfuse/sshfs";

// ── ssh config parsing ────────────────────────────────────────────────────────

/// parse ~/.ssh/config for Host aliases → user@hostname
pub fn parse_ssh_config() -> HashMap<String, String> {
    let path = dirs::home_dir()
        .unwrap_or_default()
        .join(".ssh")
        .join("config");

    let mut aliases: HashMap<String, String> = HashMap::new();
    let Ok(contents) = fs::read_to_string(&path) else {
        return aliases;
    };

    let mut current_hosts: Vec<String> = vec![];
    let mut current_hostname: Option<String> = None;
    let mut current_user: Option<String> = None;

    let flush = |hosts: &[String], hostname: &Option<String>, user: &Option<String>, out: &mut HashMap<String, String>| {
        for h in hosts {
            if h == "*" { continue; }
            let resolved = hostname.as_deref().unwrap_or(h);
            let entry = match user {
                Some(u) => format!("{u}@{resolved}"),
                None    => resolved.to_string(),
            };
            out.insert(h.clone(), entry);
        }
    };

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let Some((key, val)) = line.split_once(char::is_whitespace) else { continue; };
        match key.to_lowercase().as_str() {
            "host" => {
                flush(&current_hosts, &current_hostname, &current_user, &mut aliases);
                current_hosts    = val.split_whitespace().map(String::from).collect();
                current_hostname = None;
                current_user     = None;
            }
            "hostname" => current_hostname = Some(val.trim().into()),
            "user"     => current_user     = Some(val.trim().into()),
            _ => {}
        }
    }
    flush(&current_hosts, &current_hostname, &current_user, &mut aliases);
    aliases
}

pub fn resolve_target(arg: &str) -> String {
    let aliases = parse_ssh_config();
    aliases.get(arg).cloned().unwrap_or_else(|| arg.to_string())
}

// ── availability checks ───────────────────────────────────────────────────────

pub fn sshfs_available() -> bool {
    which("sshfs")
}

pub fn pkexec_available() -> bool {
    which("pkexec")
}

fn which(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(bin).is_file()))
        .unwrap_or(false)
}

// ── mount state ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MountInfo {
    pub mountpoint:   PathBuf,
    pub remote_home:  String,
    pub su_available: bool,
}

impl MountInfo {
    /// translate an absolute remote path to its local location under the sshfs root mount
    pub fn to_local_path(&self, remote_abs: &str) -> PathBuf {
        let rel = remote_abs.trim_start_matches('/');
        self.mountpoint.join(rel)
    }

    /// reverse: given a path under the mountpoint, recover the original remote absolute path
    pub fn to_remote_path(&self, local: &Path) -> String {
        match local.strip_prefix(&self.mountpoint) {
            Ok(rel) => format!("/{}", rel.to_string_lossy()),
            Err(_)  => local.to_string_lossy().into_owned(),
        }
    }
}

// ── /proc/mounts check ────────────────────────────────────────────────────────

fn is_mounted(mountpoint: &Path) -> bool {
    fs::read_to_string("/proc/mounts")
        .map(|m| m.contains(&mountpoint.to_string_lossy().as_ref()))
        .unwrap_or(false)
}

fn clear_stale_mount(mountpoint: &Path) -> Result<()> {
    if !mountpoint.exists() { return Ok(()); }
    if is_mounted(mountpoint) {
        unmount(mountpoint);
        if is_mounted(mountpoint) {
            return Err(anyhow!(
                "stale mount stuck at {}, try: fusermount -u {}",
                mountpoint.display(), mountpoint.display()
            ));
        }
    }
    // verify we can write to it
    let test = mountpoint.join(".confy_write_test");
    fs::write(&test, b"").map_err(|_| anyhow!(
        "can't access {} (leftover mount?), try: fusermount -u {} && rm -rf {}",
        mountpoint.display(), mountpoint.display(), mountpoint.display()
    ))?;
    let _ = fs::remove_file(&test);
    Ok(())
}

// ── resolve remote $HOME ──────────────────────────────────────────────────────

pub fn resolve_remote_home(target: &str) -> Result<String> {
    let out = Command::new("ssh")
        .args(["-o", "BatchMode=yes", "-o", "ConnectTimeout=10", target, "echo $HOME"])
        .output()
        .map_err(|_| anyhow!("ssh not found"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(anyhow!("{}", if err.is_empty() { "ssh connection failed".into() } else { err }));
    }
    let home = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if home.is_empty() { return Err(anyhow!("remote returned empty $HOME")); }
    Ok(home)
}

// ── mount / unmount ───────────────────────────────────────────────────────────

/// mount target's root filesystem at mountpoint. returns MountInfo on success.
pub fn mount(target: &str, mount_root: &Path) -> Result<MountInfo> {
    if !sshfs_available() {
        return Err(anyhow!("sshfs not found. install it: {SSHFS_URL}"));
    }

    let remote_home = resolve_remote_home(target)
        .map_err(|e| anyhow!("couldn't reach {target}: {e}"))?;

    let mount_name = target.replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '.', "_");
    let mountpoint = mount_root.join(&mount_name);
    fs::create_dir_all(&mountpoint)?;
    clear_stale_mount(&mountpoint)?;

    let remote_path  = format!("{target}:/");
    let base_opts    = "reconnect,ServerAliveInterval=15,ServerAliveCountMax=3";

    // try with allow_root first (needed for :su through the mount)
    let result = Command::new("sshfs")
        .args([&remote_path, &mountpoint.to_string_lossy().into_owned(), "-o", &format!("{base_opts},allow_root")])
        .output();

    let (su_available, mounted_ok) = match result {
        Ok(o) if o.status.success() => (true, true),
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr);
            if err.contains("allow_root") || err.contains("user_allow_other") {
                // fallback without allow_root
                let r2 = Command::new("sshfs")
                    .args([&remote_path, &mountpoint.to_string_lossy().into_owned(), "-o", base_opts])
                    .output()?;
                (false, r2.status.success())
            } else {
                return Err(anyhow!("sshfs failed: {}", err.trim()));
            }
        }
        Err(e) => return Err(anyhow!("sshfs error: {e}")),
    };

    if !mounted_ok {
        return Err(anyhow!("sshfs failed to mount {target}"));
    }

    Ok(MountInfo { mountpoint, remote_home, su_available })
}

pub fn unmount(mountpoint: &Path) -> bool {
    // try fusermount first, fall back to umount
    for cmd in [["fusermount", "-u"], ["umount", ""]] {
        let mut c = Command::new(cmd[0]);
        if !cmd[1].is_empty() { c.arg(cmd[1]); }
        c.arg(mountpoint);
        if c.output().map(|o| o.status.success()).unwrap_or(false) {
            return true;
        }
    }
    false
}

// ── remote format check ───────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum RemoteFormat {
    Ok,
    Legacy,
    Empty,
    Unreachable,
}

pub fn check_remote_format(confy_dir: &Path) -> RemoteFormat {
    let Ok(entries) = fs::read_dir(confy_dir) else {
        return RemoteFormat::Unreachable;
    };
    let names: Vec<_> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    if names.iter().any(|n| n == "config.json")  { return RemoteFormat::Ok; }
    if names.iter().any(|n| n == "tracked.json") { return RemoteFormat::Legacy; }
    RemoteFormat::Empty
}
