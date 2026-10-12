# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Live binding resolution: tmux entries are verified against a probe server
  running the user's `tmux.conf` (rewritten to real chords, dropped when
  unbound); Herdr entries resolve from `--default-config` merged with the
  user's `config.toml`. Prefix/leader keys in titles resolve from the user's
  tmux/herdr/Neovim configs.
- All-sheets view groups rows by app ("tmux · Panes") instead of unlabeled
  per-row badges.
- Omarchy shell bar widget (`omarchy plugin add`): launches the app, or offers
  the AUR install when missing.
- CONTRIBUTING.md with the "real bindings only" rule for new apps.

### Fixed

- Hyprland bindings no longer clash with stock Omarchy (tmux moved to
  SUPER+ALT+T; SUPER+T is the stock float toggle).

### Changed

- tmux sheet shows both configured prefixes (`Ctrl+Space / Ctrl+b`).

## [0.1.0] - 2026-10-12

### Added

- Initial release: pretty, searchable cheatsheet viewer for Omarchy Linux
  (Rust backend, Qt6/QML frontend via cxx-qt).
- Curated sheets for Neovim (LazyVim), tmux, Herdr, Zellij, lazygit,
  lazydocker, and btop, grouped by category in relevancy order.
- Type-to-filter fuzzy-lite search with match scoring (consecutive and
  word-boundary bonuses), live match counter, and empty state.
- Sidebar with per-tool Nerd Font icons; `--section <name>` to open scoped,
  `--list-sections` for scripting.
- Availability detection: sheets whose `command` is not in PATH are hidden
  (e.g. Zellij appears only after it is installed).
- User overlays: TOML files in `~/.config/omarchy-cheatsheets/` merge over the
  embedded sheets (replace entries by identical keys, append new sections or
  whole sheets).
- Tokyo Night palette matching the Omarchy theme, translucent window with
  Hyprland blur, logical-pixel sizing that follows the compositor's monitor
  scale on HiDPI displays.
