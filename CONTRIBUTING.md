# Contributing to omarchy-cheatsheets

Thanks for helping! This document is the short path to a good PR.

## Gates (must pass, same as CI)

```bash
cargo fmt --all -- --check
git diff --check
cargo clippy --workspace --all-targets -- -D warnings
cargo tf        # full test suite (cargo nextest run --workspace -P full)
cargo deny check
```

Every behavior change needs a focused regression test, and a CHANGELOG entry
under `## [Unreleased]`. Integration tests live in `tests/suite/` (one test
binary; declare each file with `mod name;` in `tests/suite/mod.rs`).

## Architecture at a glance

```
data/*.toml      curated sheets (embedded into the binary)
src/sheet.rs     data model (Sheet → Section → Entry, optional probe)
src/load.rs      TOML parse, overlay merge, availability probe, prefix resolution
src/resolve.rs   live binding resolution (tmux list-keys probe, herdr [keys])
src/search.rs    fuzzy-lite matcher + scoring, global rank() for quick search
src/bridge.rs    cxx-qt QObject exposed to QML (thin; no logic)
qml/Main.qml     browser UI (sidebar + grouped list)
qml/QuickSearch.qml  search-only window (--search), flat ranked hits
```

## Adding a new app's sheet

### Rule zero: real bindings only

A sheet must reflect the bindings that actually work on the user's machine.
Do not add an app whose sheet is a static copy of upstream defaults when the
tool lets users rebind keys. Every new app needs one of:

1. **A live resolver** (preferred) — tmux boots a throwaway server with the
   user's config and reads `list-keys`; herdr merges `--default-config` with
   the user's `config.toml`. Add one in `src/resolve.rs` and annotate entries
   with `probe = { ... }` so entries are rewritten to the real keys, or dropped
   when unbound.
2. **A documented exception** — the tool's keys are not configurable
   (e.g. btop), so defaults are the truth. Say so in the sheet and in your PR.

### Static sheet (no resolver needed)

1. Create `data/<app>.toml`:

   ```toml
   name = "myapp"                  # merge key, also matches overlays
   title = "MyApp"                 # sidebar title; "{prefix}" is substituted
   icon = "\uf489"                 # Nerd Font glyph
   command = "myapp"               # sheet hidden when not in PATH

   [[sections]]
   name = "Panes"                  # groups, in relevancy order

   [[sections.entries]]
   keys = ["prefix", "x"]
   description = "Close pane"
   tags = ["daily"]
   probe = { command = "kill-pane", table = "prefix" }   # when a resolver exists
   ```

2. Register it in `src/load.rs` (`EMBEDDED`).
3. Add tests: the `embedded_sheets_parse` suite covers the new sheet; resolver
   parsers get pure-function tests (fixtures as strings, no processes).

### Probe contract

- `command` + `table` (tmux): entry keys are rewritten to the real chord found
  in `tmux list-keys`; entry dropped when nothing matches. `family = true`
  keeps the curated keys when at least one binding matches (arrow families).
- `action` (herdr): resolved from `[keys]`; `prefer = "alt"` picks among
  alternates; empty binding in user config = unbound = entry dropped.
- Prefix: titles may contain `{prefix}`, resolved from the user's config
  (tmux `prefix`/`prefix2`, herdr `keys.prefix`, nvim `mapleader`).

## Release

Maintainers only: `bin/release X.Y.Z` runs the gates, bumps, and tags without
pushing. `git push --follow-tags` publishes tarballs + AUR packages via
GitHub Actions.
