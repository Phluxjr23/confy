<p align="center">
  <img src="branding/confy-full-color-transparent.png" alt="confy logo" width="256">
</p>

<h1 align="center">confy</h1>

<p align="center">a config manager for linux/unix based systems.</p>

<p align="center">simple tui for keeping track of all your config files in one place. no more hunting through ~/.config.</p>

---

## features

* **organize with groups** - create folders to organize your configs (hyprland/, nvim/, etc)
* **collapsible groups** - expand/collapse groups to keep your view clean
* **search** - real-time fuzzy search through all your configs
* **multiple sort modes** - sort by name, date modified, or file size
* **open in $EDITOR** - edit files with one keypress
* **elevated editing** - `:su` opens the selected file with root via polkit
* **remote profiles** - `:device <host>` mounts and browses another machine's tracked configs over sshfs
* **live preview pane** - toggle a side-by-side file preview with `p`
* **built-in themes** - catppuccin, dracula, gruvbox, nord, tokyo-night, one-dark, switch with `:theme`
* **remembers last file** - quick access to recently edited configs
* **built-in file picker** - navigate with vim keys, no external dependencies
* **rollback** - automatic compressed backups before every edit, restore with `:rb`
* **custom colors** - set colors via config.json, supports hex and named colors
* **vim-style keybinds** - j/k navigation, command mode
* **git integration** - sync your dotfiles to a git repo with `:git`, commit and push without leaving confy
* **streamer mode** - hide paths and timestamps with `:streamer` so nothing leaks on stream
* **lightweight and fast** - single native binary, zero runtime dependencies

## installation

### from AUR (arch linux)
```bash
yay -S confy-tui
```

### from nixpkgs (unstable)
```bash
nix-shell -I nixpkgs=https://github.com/nixos/nixpkgs/archive/nixos-unstable.tar.gz -p confy-tui
```

[confy is in nixpkgs](https://github.com/NixOS/nixpkgs/pull/543546#event-28187717813) under the name `confy-tui`.

### from cargo
```bash
cargo install confy-tui
```

### build from source
```bash
git clone https://github.com/phluxjr/confy.git
cd confy
cargo build --release
sudo install -Dm755 target/release/confy /usr/local/bin/confy
# optionally install the man page
sudo install -Dm644 confy.1 /usr/share/man/man1/confy.1
```

## dependencies

none required for core functionality.

two commands need optional system tools:
* `:device` (remote profiles) needs [`sshfs`](https://github.com/libfuse/sshfs) installed
* `:su` (elevated editing) needs `pkexec` (polkit), and for use with `:device`, `user_allow_other` set in `/etc/fuse.conf` on your local machine
* `:git commit` / `:git push` need `git` installed

confy will tell you clearly if any of these are missing rather than crashing.

## usage

just run `confy` in your terminal.

### navigation

* `j/k` or `arrow keys` - move up/down
* `enter` - open file in $EDITOR (or toggle group)
* `space` - toggle group expand/collapse
* `p` - toggle live preview pane
* `/` - search mode
* `:` - command mode
* `q` - quit

### commands

#### file management
* `:ac` - add config to ungrouped
* `:ac <group>` - add config to specific group
* `:rm` - remove selected file from tracking (does not delete the file)
* `:l` - open last edited file
* `:rb` - rollback selected file to last backup
* `:su` - open selected file with root via pkexec (needs polkit)

#### remote profiles
* `:device <host>` or `:ssh <host>` - mount and browse a remote host's tracked configs over sshfs (accepts an `~/.ssh/config` alias or a literal `user@host`)
* `:device local` - switch back to your local configs
* `:device` (no args) - show the currently active device, if any

#### git integration
* `:git sync` - copy all tracked configs into your dotfiles repo (organized by group)
* `:git commit` - sync + prompt for a commit message
* `:git push` - sync + commit + push to remote
* `:git dir` - change the dotfiles repo path interactively
* `:git dir <path>` - set the dotfiles repo path directly
* `:git reset` - reset dotfiles repo path to `~/dotfiles-git`
* `:bl` - toggle git blacklist on the selected file or group (blacklisted items are skipped by `:git sync`)
* `:bouncer` - open a menu to bulk-manage the git blacklist (space to toggle, Z to confirm)

#### appearance
* `:theme <name>` - switch color theme (`catppuccin`, `dracula`, `gruvbox`, `nord`, `tokyo-night`, `one-dark`)
* `:theme` (no args) - list available themes
* `:streamer` - toggle streamer mode (hides paths and timestamps)

#### group management
* `:ag <group>` - add new group
* `:mg <group>` - move selected file to group
* `:rg <group>` - remove group (moves files to ungrouped)

#### sorting & filtering
* `:sort name` - sort alphabetically
* `:sort date` - sort by last modified
* `:sort size` - sort by file size
* `:reverse` - toggle ascending/descending order
* `/` then type - search files and groups in real-time

#### configuration
* `:cd` - change config directory (opens built-in file picker)
* `:cd <path>` - set config directory directly
* `:cd reset` - reset to ~/.config
* `:q` - quit

### rollback

confy automatically saves a compressed backup of any file to `/tmp/<filename>.confbak` before you open it for editing. if you make a mess of your config, select the file and run `:rb` to restore it.

rollback can be disabled in config.json:
```json
"settings": {
  "rollback": false
}
```

### git integration

`:git sync` copies your tracked configs into `~/dotfiles-git` (or wherever `git_dir` points), organized by group name. files in named groups go into a folder with the group's name. ungrouped files with a shared filename prefix (e.g. `hyprland.conf` and `hyprlock.conf`) get auto-sorted into a folder named after that prefix.

`:git commit` and `:git push` sync first, then prompt for a commit message in the terminal. `:git push` requires at least one prior commit to exist. make your initial commit manually.

use `:bl` on any file or group to exclude it from syncing (useful for `.env` files or anything sensitive). `:bouncer` gives you a full menu to manage the blacklist in bulk.

git settings in config.json:
```json
"git_dir": "~/dotfiles-git",
"git_blacklist": ["/home/user/.config/some/secret.env"],
"git_blacklist_groups": ["personal"],
"git_auto_push": false,
"git_auto_commit": false
```

### themes

confy ships with six built-in themes: `catppuccin` (default), `dracula`, `gruvbox`, `nord`, `tokyo-night`, and `one-dark`. switch with:
```
:theme dracula
```
applies instantly and persists to config.json, no restart needed. run `:theme` with no arguments to list all available themes.

### streamer mode

`:streamer` toggles streamer mode, which hides full file paths and timestamps. only filenames are shown. useful if you're on stream and don't want your username or directory structure visible. persists to config.json, or set it directly:
```json
"settings": {
  "streamer_mode": true,
  "streamer_hide_git": true
}
```

### preview pane

press `p` to toggle a live preview pane alongside your file list. it shows the first lines of the selected file, updating as you move the selection. works correctly on a mounted `:device` too. your preference persists across restarts.

### remote profiles (`:device`)

`:device <host>` mounts a remote machine's entire filesystem over sshfs and switches confy's view to that host's tracked configs, letting you browse and edit them exactly like local files.

```
:device phluxjr           # resolves an alias from ~/.ssh/config
:device phluxjr@exam.ple  # or connect directly
:device local             # switch back to your own configs
```

requires [`sshfs`](https://github.com/libfuse/sshfs) installed locally. if the remote host is running an older confy that predates `config.json` (i.e. still on `tracked.json`), confy will refuse to connect and ask you to update confy there first.

while on a device, the header shows `[remote: <host>]` so it's always clear you're not looking at your local files. edits, previews, sorting, and rollback all work through the mount.

### elevated editing (`:su`)

select a file and run `:su` to open it with root via `pkexec`. works on local files and on files viewed through a mounted `:device`.

needs `pkexec` (polkit) installed. for `:su` to work while a device is mounted, your local machine also needs `user_allow_other` uncommented in `/etc/fuse.conf`. confy will tell you if it's missing.

### search mode

press `/` to enter search mode, then start typing:
* filters both files and groups in real-time
* case-insensitive matching
* `enter` to confirm, `esc` to clear and show all files

### groups

groups are purely organizational. your actual config files stay in their original locations. `ungrouped` always appears at the bottom of the list regardless of sort mode.

groups are collapsible, press `space` or `enter` on a group header to toggle.

## configuration file

confy stores everything in `~/.config/confy/config.json`. upgrading from an older version with `tracked.json`? confy migrates it automatically on first run.

full example config.json:
```json
{
  "groups": {
    "hyprland": ["/home/user/.config/hypr/hyprland.conf"],
    "nvim": ["/home/user/.config/nvim/init.lua"],
    "ungrouped": []
  },
  "settings": {
    "rollback": true,
    "theme": "catppuccin",
    "colors": {
      "bg": "default",
      "fg": "default",
      "highlight": "#cba6f7",
      "group": "#89b4fa"
    },
    "background_enable": false,
    "background_color": "#1e1e2e",
    "streamer_mode": false,
    "streamer_hide_git": true,
    "editor": null,
    "ssh_allow": true
  },
  "git_dir": "/home/user/dotfiles-git",
  "git_blacklist": [],
  "git_blacklist_groups": [],
  "git_auto_push": false,
  "git_auto_commit": false,
  "preview_enabled": false
}
```

### settings reference

| key | description | default |
| --- | --- | --- |
| `rollback` | save a backup before every edit | `true` |
| `theme` | active color theme | `"catppuccin"` |
| `background_enable` | draw a solid background instead of terminal default | `false` |
| `background_color` | background color (hex) | `"#1e1e2e"` |
| `streamer_mode` | hide paths and timestamps in the file list | `false` |
| `streamer_hide_git` | also hide git-blacklisted files in streamer mode | `true` |
| `editor` | override `$EDITOR` (useful for .desktop launchers) | `null` |
| `ssh_allow` | enable `:device` / sshfs integration | `true` |

## why confy?

tired of doing `cd ~/.config/whatever` a million times a day? same. confy keeps all your important configs in one list so you can jump to them instantly.

organize related configs into groups, search through everything, sort however you want, and open files in your editor with a single keypress. if you break something, roll it back. if you want your dotfiles in git, `:git push` and you're done.

simple, fast, does one thing well.

## examples
```
# start confy
confy

# create some groups and add configs
:ag hyprland
:ac hyprland

# move a file between groups
:mg shell

# search for configs
/hypr

# sort by recently modified, newest first
:sort date
:reverse

# oops, broke your config
:rb

# switch theme
:theme tokyo-night

# toggle preview pane
p

# check on your server's configs
:device phluxjr
:su
:device local

# sync dotfiles to git
:git push

# going live? hide your paths
:streamer
```

## tips

* set `export EDITOR=nvim` in your shell rc, or use the `editor` setting in config.json
* use groups to organize by application (hyprland/, nvim/, kitty/)
* use `:sort date` to quickly find recently edited configs
* use `:bl` on `.env` files or anything sensitive before running `:git sync`
* collapse groups you don't use often to keep the view clean
* missing files show up in red so you know when a config has moved

---

<p align="center">
  <strong>copyright &copy; 2025-2026 phluxjr</strong><br>
  GPL-3.0-or-later
</p>

<p align="center">
  prs welcome! if you have ideas for improvements, open an issue or submit a pr.
</p>

<p align="center">
  <em>man page included - <code>man confy</code> after install</em>
</p>
