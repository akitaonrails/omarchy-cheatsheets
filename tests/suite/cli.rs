//! CLI parsing and window selection: `--search` opens the quick-search window.

use clap::Parser;
use omarchy_cheatsheets::cli::{Cli, Window};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("omarchy-cheatsheets").chain(args.iter().copied()))
}

#[test]
fn search_flag_takes_an_optional_query() {
    assert_eq!(parse(&["--search"]).unwrap().search.as_deref(), Some(""));
    assert_eq!(
        parse(&["--search", "split pane"])
            .unwrap()
            .search
            .as_deref(),
        Some("split pane")
    );
    assert_eq!(
        parse(&["--search=quit"]).unwrap().search.as_deref(),
        Some("quit")
    );
    assert_eq!(parse(&[]).unwrap().search, None);
}

#[test]
fn search_conflicts_with_section_and_list_sections() {
    assert!(parse(&["--search", "--section", "nvim"]).is_err());
    assert!(parse(&["--search", "--list-sections"]).is_err());
}

#[test]
fn window_follows_search_flag() {
    assert_eq!(parse(&["--search"]).unwrap().window(), Window::QuickSearch);
    assert_eq!(parse(&[]).unwrap().window(), Window::Browser);
    // Regression: --section keeps opening the full browser.
    assert_eq!(
        parse(&["--section", "tmux"]).unwrap().window(),
        Window::Browser
    );
}

#[test]
fn every_window_points_at_a_qml_file_in_the_tree() {
    for window in [Window::Browser, Window::QuickSearch] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(window.qml_path());
        assert!(path.is_file(), "missing {}", path.display());
        assert!(
            window.qml_url().ends_with(window.qml_path()),
            "{} does not serve {}",
            window.qml_url(),
            window.qml_path()
        );
    }
}
