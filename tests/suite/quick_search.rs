//! Quick search over the real embedded sheets: guards against data changes
//! that would make common lookups come back empty.

use omarchy_cheatsheets::load;
use omarchy_cheatsheets::search;

#[test]
fn words_in_any_order_find_real_tmux_split() {
    let sheets = load::load_all(tempfile::tempdir().unwrap().path()).unwrap();
    let hits = search::rank(&sheets, "pane split", 50);
    assert!(
        hits.iter()
            .any(|h| h.sheet == "tmux" && h.entry.description.starts_with("Split pane")),
        "tmux split entries missing from {hits:?}"
    );
}

#[test]
fn ranked_hits_span_multiple_sheets() {
    let sheets = load::load_all(tempfile::tempdir().unwrap().path()).unwrap();
    let hits = search::rank(&sheets, "quit", 50);
    let mut apps: Vec<_> = hits.iter().map(|h| h.sheet.as_str()).collect();
    apps.sort_unstable();
    apps.dedup();
    assert!(
        apps.len() > 1,
        "expected hits from several apps, got {apps:?}"
    );
    assert!(hits.windows(2).all(|w| w[0].score >= w[1].score));
}

#[test]
fn section_name_finds_its_real_entries() {
    let sheets = load::load_all(tempfile::tempdir().unwrap().path()).unwrap();
    let hits = search::rank(&sheets, "copy mode vi", 50);
    assert!(
        hits.iter()
            .any(|h| h.sheet == "tmux" && h.section == "Copy mode (vi)"),
        "tmux copy-mode section missing from {hits:?}"
    );
}

#[test]
fn app_name_narrows_to_that_app() {
    let sheets = load::load_all(tempfile::tempdir().unwrap().path()).unwrap();
    let hits = search::rank(&sheets, "lazygit quit", 50);
    assert_eq!(hits.first().map(|h| h.sheet.as_str()), Some("lazygit"));
}
