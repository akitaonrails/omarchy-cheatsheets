# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `~/.config/omarchy-cheatsheets/settings.toml` stores appearance settings
  (`theme`, `font_family`, `font_size`, `opacity`); out-of-range values are
  clamped. The name is reserved: the overlay loader skips it.
- Palettes come from Omarchy theme `colors.toml` files (stock and
  `~/.config/omarchy/themes`); missing colors fall back to Tokyo Night.
- Appearance drawer (gear button or `Ctrl+,`): theme cards previewing each
  theme's own colors (arrow keys switch live), a font picker that renders
  each family in itself, a live preview row, a keycap-style size stepper and
  an opacity slider, all in the app's palette. Changes apply live and are
  saved to `settings.toml`; the footer shows where. `Ctrl+=` / `Ctrl+-` /
  `Ctrl+0` change the font size; the whole layout scales with it.

### Changed

- The window follows the active Omarchy theme instead of always using Tokyo
  Night (unchanged look on the stock theme). Esc closes the appearance panel
  before it closes the app.

## [0.1.1] - 2026-10-12

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
- Internal: prefix/leader resolution moved from `load.rs` to `resolve.rs`;
  the tmux/herdr rewrite-and-drop logic now has direct unit tests.

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
