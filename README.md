# omarchy-cheatsheets

Pretty, searchable cheatsheets for [Omarchy](https://omarchy.org/) Linux.
A small Qt6/QML window backed by Rust (cxx-qt): launch it, start typing,
the list narrows instantly. Ships curated sheets for **Neovim (LazyVim)**,
**tmux**, **Herdr**, **Zellij**, **lazygit**, **lazydocker**, and **btop** —
tools not installed on your system are hidden automatically.

<!-- screenshot -->

## Install

```bash
yay -S omarchy-cheatsheets-bin   # prebuilt binary (AUR)
yay -S omarchy-cheatsheets       # build from source (AUR)
```

## Usage

```bash
omarchy-cheatsheets                  # all sheets
omarchy-cheatsheets --section nvim   # open scoped to one tool
omarchy-cheatsheets --list-sections  # machine-readable list of sheets
```

Suggested Hyprland bindings (`~/.config/hypr/bindings.lua`):

```lua
o.bind("SUPER + N", "Neovim cheatsheet", { launch = "omarchy-cheatsheets --section nvim" })
o.bind("SUPER + T", "Tmux cheatsheet", { launch = "omarchy-cheatsheets --section tmux" })
o.bind("SUPER + CTRL + SLASH", "Cheatsheets", { launch = "omarchy-cheatsheets" })
```

and a floating window rule (`~/.config/hypr/looknfeel.lua`):

```lua
o.window("^(omarchy-cheatsheets)$", { float = true, center = true, size = { 980, 660 } })
```

## Customizing sheets

Drop TOML files in `~/.config/omarchy-cheatsheets/`. Same sheet name + same
section + same `keys` replaces an entry; anything else is appended, so you can
reword bindings, add sections, or add whole new programs:

```toml
name = "tmux"
command = "tmux"      # optional: sheet hidden when not in PATH
icon = "\uf489"       # optional: Nerd Font glyph for the sidebar

[[sections]]
name = "Panes"

[[sections.entries]]
keys = ["prefix", "x"]
description = "Close pane"
tags = ["daily"]      # optional, free-form
```

## Development

Requirements: Rust 1.95 (pinned in `rust-toolchain.toml`), Qt6
(`qt6-base`, `qt6-declarative`), clang, cmake.

```bash
cargo t                                    # everyday test loop (nextest)
cargo tf                                   # full suite, pre-push
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo deny check
```

Layout: `src/sheet.rs` (model), `src/load.rs` (embedded + overlay merge,
availability probe), `src/search.rs` (scoring), `src/bridge.rs` (cxx-qt
QObject), `qml/Main.qml` (all UI), `data/*.toml` (curated sheets).
Rules and gates live in [AGENTS.md](AGENTS.md).

## Release

```bash
bin/release 0.1.1   # gates + version bump + changelog + tag (does not push)
git push --follow-tags
```

Tagging `vX.Y.Z` runs `.github/workflows/release.yml`: builds x86_64 and
aarch64 tarballs, creates the GitHub release, and publishes both AUR packages
(when the `AUR_SSH_PRIVATE_KEY` secret is set).
