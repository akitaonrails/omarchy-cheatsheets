# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- PageUp and PageDown keyboard scrolling across the bindings list in both the
  browser and quick-search windows, while keeping focus in the search field.

## [0.4.0] - 2026-10-12

### Added

- Hyprland/Omarchy sheet, resolved live from the running compositor
  (`hyprctl binds`): user's Lua overrides shown, unbound entries dropped.
- `--install-bindings`: optional SUPER+K takeover (backs up `bindings.lua`,
  idempotent, opens the Hyprland sheet) — a drop-in replacement for the stock
  keybindings menu. AUR packages print the hint on install/upgrade.

### Changed

- Default base font size is now 16 (was 14); the whole layout scales from it.
  `Ctrl+0` resets to the real default instead of the design base.

## [0.3.0] - 2026-10-12

### Added

- Sheets for kitty, Alacritty, and Ghostty, resolved live from their config
  files over upstream defaults: remapped keys show the user's chords, unmapped
  or `clear_all_shortcuts`-dropped defaults disappear. Hidden when the
  terminal isn't installed.
- Tab / Shift+Tab cycles sheets in the browser window; the search field keeps
  focus.

### Fixed

- `display_chord` treated single-letter keys as tmux modifier shorthand, so
  kitty's `ctrl+shift+c` rendered as "Ctrl Shift Ctrl". Modifier names now
  apply only before the final key.

## [0.2.0] - 2026-10-12

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
  The window color is premultiplied, so lowering the opacity shows the
  desktop through on light themes too instead of washing out to white.
  The opacity slider has a real hit area (it was laid out 0px tall, so its
  handle could not be dragged).

### Changed

- The window follows the active Omarchy theme instead of always using Tokyo
  Night (unchanged look on the stock theme). Esc closes the appearance panel
  before it closes the app.

- Quick search window (`--search [QUERY]`): one field, one flat list ranked
  across every sheet with the app and section on each row; ↑/↓ to move, Esc
  to close. Opens with the same window class and size as the browser.

### Changed

- Bar widget: left click opens the quick search, right click the full
  browser. An older binary without `--search` falls back to the browser.
- Search matches query words in any order ("pane split" finds "Split pane");
  every word must still match. Queries that matched before still match.
- Search also matches an entry's section and app name ("copy mode",
  "tmux quit"), in both windows. Entries whose own text matches rank first.

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
