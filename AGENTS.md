# AGENTS.md — omarchy-cheatsheets

Pretty, searchable Neovim and tmux cheatsheets for Omarchy Linux. Rust backend +
QML (Qt6) frontend via cxx-qt. Single crate; keep it small.

## Tech stack

- Rust edition 2024, toolchain 1.95 (pinned in `rust-toolchain.toml`).
- Qt6 via `cxx-qt` 0.10 / `cxx-qt-lib`; QML embedded via Qt resources (`build.rs`).
- Data: curated TOML sheets in `data/`, embedded with `include_str!`; user overlays
  from `~/.config/omarchy-cheatsheets/*.toml` merge on top at runtime.

## Required gates (run before committing; same as CI)

```bash
cargo fmt --all -- --check
git diff --check
cargo clippy --workspace --all-targets -- -D warnings
cargo tf        # = cargo nextest run --workspace -P full
cargo deny check
```

## Code style

- Match the surrounding file's conventions. Comments explain *why*, never restate *what*.
- Small, scoped, behavior-preserving changes. No speculative abstractions or public
  surface without a caller.
- Prefer explicit fallbacks over `unwrap`, `expect`, or `unreachable!` in runtime paths;
  panics are acceptable in tests only.
- `unsafe` is forbidden by workspace lint; do not add it.
- `main.rs` is a shim: parse args, call library functions, render/exit.
- Errors: thiserror enums in the library (`sheet.rs`), anyhow only at the binary boundary.
- Keep QML thin: all filtering/search logic lives in Rust (`search.rs`), QML renders.

## Testing

- `cargo t` for the everyday loop (skips `slow*`/`stress*` modules); `cargo tf` before push.
- Add focused regression tests for every bug fix and behavior change.
- Filesystem tests use `tempfile` and injected roots; never depend on the real user home.
- Integration tests live in `tests/suite/` — one test binary, each file declared with
  `mod name;` in `tests/suite/mod.rs`.

## Project maintenance

- SemVer + Keep a Changelog: every change lands an entry under `## [Unreleased]`.
- No version bumps or release tags without explicit user approval.
- Do not add dependencies without checking the manifest doesn't already have the capability.

### Cleanup procedures (inherited from ai-memory house rules — mandatory, not optional)

- **After every release** (tagged + artifacts published): run `cargo clean` in the repo
  and in any release worktree / extra `CARGO_TARGET_DIR` created along the way, then
  remove finished `git worktree`s. Treat it as part of the release, like the tag itself.
- **After any release, deploy verification, or Docker-based test**: surgically remove the
  images, containers, build cache, and /tmp scratch that session created. Never
  blanket-prune a shared Docker daemon.
- **After merging a PR or closing an issue**: clean up scratch artifacts the work created —
  remove git worktrees (`git worktree remove --force` after verifying zero uncommitted
  files, then `git worktree prune`) and delete temp dirs/logs the work left in /tmp.
  Never touch other projects' files or anything not attributable to the session.
- **During normal development**: `cargo sweep --time 7` weekly is the lighter-touch
  equivalent of a full clean.
