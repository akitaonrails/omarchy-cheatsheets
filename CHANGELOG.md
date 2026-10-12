# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
