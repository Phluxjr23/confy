# the history of confy

confy has come a long way from its first release in november all the way to being rewritten in rust and getting into the AUR and nixpkgs. here's how all that happened!

---

## v1.0.0 - the initial commit

**november 9th, 2025**

confy started as a [199 line python script](https://github.com/phluxjr/confy/commit/0a6d27abcddd70a4c1d12c27b5ccf579982f2b92#diff-b10564ab7d2c520cdd0243874879fb0a782862c3c902ab535faabe57d5a505e1) written in very, VERY bad curses — and honestly it was surprising it even worked given the author's prior experience with the curses library (read: basically none).

used ranger as a file picker, no configurable colors, pure proof-of-concept material. but it worked, and that was enough.

---

## v2.0.0 - i really thought "new year" was an actual reason for this monstrosity huh

**january 6th, 2026**

"monstrosity" isn't entirely fair — this version introduced some genuinely useful stuff:

- changeable config directory
- fuzzy search
- groups
- windows support (basic, questionable, don't ask)

not much of note compared to what came later, but it was a real step up from the proof-of-concept.

---

## v2.1.0 - another time-based excuse

**march 7th, 2026**

the excuse this time was "bi-monthly update time." regardless, this one actually mattered:

- removed ranger as a dependency (did we mention it removed ranger?)
- custom colors
- man page
- rollback
- overall just... better

seriously though. no more ranger.

---

## v2.1.2 - huh?

**date: no thanks, i'm good**

the entire release description was "add github actions builds" and nothing else. linus torvalds would steal someone's kneecaps for that commit message. it added github actions builds.

---

## v2.2.0 - bet you weren't expecting another!

**april 23rd, 2026**

a genuine QoL update:

- `:cd` (re-added properly this time)
- dynamic pages instead of the stupid hardcoded 10 line limit
- first-startup tutorial

---

## v3.0.0 - finally no more v2s

**july 12th, 2026**

taking directly from the release notes because they're actually good this time:

> have you ever felt like confy was a little..barebones? like there's no way to preview a file beforehand! or having to ssh into your server and run confy there! well feel no more! with confy v3.0.0, it's now a PROPER tool for terminal dwellers like myself!

this release added:

- live preview pane (toggle with `p`, shows file contents or group info)
- remote profiles via sshfs (`:device <host>`)
- elevated editing via polkit (`:su`)
- built-in themes: catppuccin, dracula, gruvbox, nord, tokyo-night, one-dark
- community themes teased (coming soon...)

also around this time: confy landed in [nixpkgs](https://github.com/NixOS/nixpkgs/pull/543546) and the AUR under the name `confy-tui` (thanks, some gnome conference manager).

---

## v3.1.0 - v3 vs v3, japan

**august 10th, 2026**

the big one. confy was rewritten from scratch in rust using ratatui, going from a single 1503 line python file to 1765 lines across 8 well-organised modules. the binary is now fully self-contained with zero runtime dependencies.

new in this release:

- **full rust rewrite** - single native binary, no python required
- **git integration** - sync your dotfiles to a repo with `:git sync`, `:git commit`, `:git push`
- **git blacklist** - exclude files or entire groups from syncing with `:bl` and `:bouncer`
- **streamer mode** - hide paths and timestamps with `:streamer` so nothing leaks on stream
- **new config options** - `editor` override, `ssh_allow`, `background_enable`, `background_color`, `streamer_hide_git`
- **proper table layout** - filename, date, and directory columns that actually line up
- **ungrouped always last** - regardless of sort mode
- **panic-safe terminal cleanup** - no more mangled shells on crash
- **unicode-safe preview** - files with multibyte characters no longer crash the preview pane
- **available on cargo** - `cargo install confy-tui`

the rewrite increased the line count but the codebase is now split into logical modules (`app.rs`, `ui.rs`, `commands.rs`, `config.rs`, `device.rs`, `git.rs`, `picker.rs`, `tutorial.rs`) making it dramatically easier to maintain and contribute to.

---

*and that's the story so far. not bad for a tool that started as a 199 line script because hunting through `~/.config` was annoying.*
