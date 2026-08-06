<p align="center">
  <img src="confy-logo.png" alt="confy logo" width="256">
</p>

<h1 align="center">confy</h1>

<p align="center">a config manager for linux/unix based systems including macos (unix) and windows.</p>

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
* **built-in file picker** - no external dependencies, navigate with vim keys
* **rollback** - automatic compressed backups before every edit, restore with `:rb`
* **custom colors** - set colors via config.json, supports hex and named colors
* **vim-style keybinds** - j/k navigation, command mode
* **lightweight and fast** - pure python with curses, zero required dependencies
* **cross-platform** - works on linux, macos, bsd, windows

## installation

### from AUR (arch linux)
```bash
yay -S confy-tui
```

### manual install
```bash
git clone https://gitlab.com/phluxjr/confy.git
cd confy
chmod +x main.py
sudo ln -s $(pwd)/main.py /usr/local/bin/confy
# optionally install the man page
sudo install -Dm644 confy.1 /usr/share/man/man1/confy.1
```

## dependencies

* python3
* curses (included with python)

that's it for core functionality, no ranger, no external tools.

two commands need optional system tools:
* `:device` (remote profiles) needs [`sshfs`](https://github.com/libfuse/sshfs) installed
* `:su` (elevated editing) needs `pkexec` (polkit), and for use with `:device`, `user_allow_other` set in `/etc/fuse.conf` on your local machine

confy will tell you clearly if either is missing rather than crashing.

## nixpkgs information

[confy is currently in nixpkgs](https://github.com/NixOS/nixpkgs/pull/543546#event-28187717813) as of ~2 weeks ago, but it should be known that *i do not have a NixOS machine that currently has confy in it's repos.* **however**, confy should be in the unstable repo under the name "`confy-tui`". you can test with:
```bash
sudo nix-channel --add https://nixos.org/channels/nixos-unstable nixos-unstable # add the unstable repo
sudo nix-channel --update # rebuild channels to include the new one
nix-shell -I nixpkgs=https://github.com/nixos/nixpkgs/archive/nixos-unstable.tar.gz -p confy-tui # go into a nix shell with confy installed
```

## usage

just run `confy` in your terminal

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

#### appearance
* `:theme <name>` - switch color theme (`catppuccin`, `dracula`, `gruvbox`, `nord`, `tokyo-night`, `one-dark`)
* `:theme` (no args) - list available themes

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

### colors

customize colors in `~/.config/confy/config.json` under `settings.colors`. values can be named colors or hex codes:
```json
"settings": {
  "colors": {
    "bg":        "default",
    "fg":        "default",
    "highlight": "#cba6f7",
    "group":     "#89b4fa"
  }
}
```

named colors: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, `default`, `lavender`, `pink`, `purple`, `orange`

hex colors require a terminal that supports 256 colors (most do).

### themes

confy ships with six built-in themes: `catppuccin` (default), `dracula`, `gruvbox`, `nord`, `tokyo-night`, and `one-dark`. switch with:
```
:theme dracula
```
this applies instantly and persists to config.json, no restart needed. run `:theme` with no arguments to list all available themes.

### preview pane

press `p` to toggle a live preview pane alongside your file list. it shows the first lines of the selected file, updating as you move the selection. the preview reads through the same path resolution as everything else, so it works correctly on a mounted `:device` too. your preference persists across restarts.

### remote profiles (`:device`)

`:device <host>` mounts a remote machine's entire filesystem over sshfs and switches confy's view to that host's tracked configs, letting you browse and edit them exactly like local files.

```
:device phluxjr           # resolves an alias from ~/.ssh/config
:device phluxjr@exam.ple  # or connect directly
:device local             # switch back to your own configs
```

requires [`sshfs`](https://github.com/libfuse/sshfs) installed locally. if the remote host is running an older confy that predates `config.json` (i.e. still on `tracked.json`), confy will refuse to connect and ask you to update confy there first, rather than guessing at an incompatible format.

while on a device, the header shows `[remote: <host>]` so it's always clear you're not looking at your local files. edits, previews, sorting, and rollback all operate on the real remote files through the mount.

### elevated editing (`:su`)

select a file and run `:su` to open it with root, via `pkexec`. this works on local files and on files viewed through a mounted `:device` alike, since `pkexec` just elevates whatever local path is currently in view.

on a mounted device, this is genuinely useful: sshfs presents remote files under your own uid, so root-owned remote files (like `/etc/pf.conf`) may be unreadable/unwritable normally. `:su` gives you local root, which can write through the mount even when your regular user can't.

this needs `pkexec` (polkit) installed. for `:su` to work while a device is mounted, your **local** machine also needs `user_allow_other` uncommented in `/etc/fuse.conf` (a one-time system config change, confy will tell you if it's missing and reconnecting the device is enough to pick it up after you set it).

### search mode

press `/` to enter search mode, then start typing:
- filters both files and groups in real-time
- case-insensitive matching
- `enter` to accept and keep filtering
- `esc` to clear search and show all files

### groups

groups are purely organizational - your actual config files stay in their original locations. groups help you organize your tracked configs into logical categories like "hyprland", "nvim", "shell", etc.

groups are collapsible - press `space` or `enter` on a group header to toggle.

## configuration file

confy stores everything in `~/.config/confy/config.json`. if you're upgrading from an older version with `tracked.json`, confy will automatically migrate it on first run.

full example config.json:
```json
{
  "groups": {
    "ungrouped": [],
    "hyprland": ["/home/user/.config/hypr/hyprland.conf"],
    "nvim": ["/home/user/.config/nvim/init.lua"]
  },
  "settings": {
    "rollback": true,
    "theme": "catppuccin",
    "colors": {
      "bg": "default",
      "fg": "default",
      "highlight": "#cba6f7",
      "group": "#89b4fa"
    }
  },
  "preview_enabled": false
}
```

## why confy?

tired of doing `cd ~/.config/whatever` a million times a day? same. confy keeps all your important configs in one list so you can jump to them instantly.

organize related configs into groups, search through everything, sort however you want, and open files in your editor with a single keypress. if you break something, roll it back.

simple, fast, does one thing well.

## examples
```bash
# start confy
confy

# create some groups
:ag hyprland
:ag nvim
:ag shell

# add configs to groups
:ac hyprland  # opens file picker, navigate to hyprland.conf
:ac nvim      # opens file picker, navigate to init.lua

# move existing files between groups
# (select file first, then)
:mg shell

# search for configs
/hypr         # shows only hyprland-related files

# sort by recently modified
:sort date
:reverse      # newest first

# oops, broke your config
:rb           # rollback to last backup

# switch to a nicer theme
:theme tokyo-night

# toggle a live preview while browsing
p

# check on your server's system configs
:device phluxjr
:su           # edit a root-owned file like /etc/pf.conf
:device local # back to your own machine
```

## tips

* set `export EDITOR=nvim` in your shell rc for your preferred editor
* use groups to organize by application (hyprland/, nvim/, kitty/)
* use `:sort date` to quickly find recently edited configs
* search with `/` to quickly jump to specific configs
* collapse groups you don't use often to keep view clean
* missing files show up in red so you know when a config has moved

## windows support

on windows, change the config directory to where you keep your configs:
```
:cd
# navigate to C:\Users\YourName\AppData\Local or wherever
```

`:device` and `:su` rely on sshfs/FUSE and polkit, both linux-specific, so they're not available on windows or macos. everything else works cross-platform.

---

<p align="center">
  <strong>copyright © 2025-2026 phluxjr</strong><br>
  GPL-3.0-or-later
</p>

<p align="center">
  prs welcome! this is a simple tool but if you have ideas for improvements, open an issue or submit a pr.
</p>

<p align="center">
  <em>man page included - <code>man confy</code> after install</em>
</p>
