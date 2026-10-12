//! Overlay merge behavior: embedded sheets must stay intact, user TOML on top.

use omarchy_cheatsheets::load::{self, LoadError};
use omarchy_cheatsheets::search;

#[test]
fn embedded_sheets_parse() {
    let sheets = load::load_all(tempfile::tempdir().unwrap().path()).unwrap();
    let names: Vec<_> = sheets.iter().map(|s| s.name.as_str()).collect();
    for expected in [
        "nvim",
        "tmux",
        "herdr",
        "zellij",
        "lazygit",
        "lazydocker",
        "btop",
    ] {
        assert!(
            names.contains(&expected),
            "missing embedded sheet {expected}"
        );
    }
    assert!(sheets.iter().all(|s| !s.sections.is_empty()));
}

#[test]
fn unavailable_commands_are_filtered_out() {
    let sheets = load::load_all(tempfile::tempdir().unwrap().path()).unwrap();
    // Inject a fake PATH probe: everything unavailable except tmux.
    let visible = load::filter_available(sheets, |cmd| cmd == "tmux");
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].name, "tmux");
}

#[test]
fn command_available_finds_real_executables() {
    assert!(load::command_available("sh"));
    assert!(!load::command_available("definitely-not-a-real-binary-xyz"));
}

#[test]
fn missing_overlay_dir_is_fine() {
    let tmp = tempfile::tempdir().unwrap();
    let sheets = load::load_all(&tmp.path().join("does-not-exist")).unwrap();
    assert_eq!(sheets.len(), 7, "embedded sheets load without overlays");
}

#[test]
fn overlay_replaces_entry_with_same_keys() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("mine.toml"),
        r##"
name = "tmux"
[[sections]]
name = "Panes"
[[sections.entries]]
keys = ["prefix", "x"]
description = "Close pane (my wording)"
"##,
    )
    .unwrap();

    let sheets = load::load_all(tmp.path()).unwrap();
    let tmux = sheets.iter().find(|s| s.name == "tmux").unwrap();
    let panes = tmux.sections.iter().find(|s| s.name == "Panes").unwrap();
    let x_entries: Vec<_> = panes
        .entries
        .iter()
        .filter(|e| e.keys == ["prefix", "x"])
        .collect();
    assert_eq!(x_entries.len(), 1, "overlay must replace, not duplicate");
    assert_eq!(x_entries[0].description, "Close pane (my wording)");
}

#[test]
fn overlay_can_add_section_and_new_sheet() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("extra.toml"),
        r##"
name = "tmux"
[[sections]]
name = "Custom"
[[sections.entries]]
keys = ["prefix", "T"]
description = "My custom binding"
"##,
    )
    .unwrap();
    std::fs::write(
        tmp.path().join("lazygit.toml"),
        r##"
name = "lazygit"
[[sections]]
name = "Basics"
[[sections.entries]]
keys = ["q"]
description = "Quit"
"##,
    )
    .unwrap();

    let sheets = load::load_all(tmp.path()).unwrap();
    let tmux = sheets.iter().find(|s| s.name == "tmux").unwrap();
    assert!(tmux.sections.iter().any(|s| s.name == "Custom"));
    assert!(sheets.iter().any(|s| s.name == "lazygit"));
}

#[test]
fn malformed_overlay_is_an_error_not_silent() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("broken.toml"), "not = [valid").unwrap();
    let err = load::load_all(tmp.path()).unwrap_err();
    assert!(matches!(err, LoadError::Parse { .. }));
}

#[test]
fn search_narrows_across_sheets() {
    let sheets = load::load_all(tempfile::tempdir().unwrap().path()).unwrap();
    let all = search::filter(&sheets, "");
    let narrowed = search::filter(&sheets, "split");
    assert!(narrowed.len() < all.len());
    assert!(
        narrowed
            .iter()
            .flat_map(|s| &s.entries)
            .any(|e| e.entry.description.contains("Split"))
    );
}
