//! CLI definition: thin arg parsing; the binary shim calls into this.

use clap::Parser;

/// Pretty, searchable Neovim and tmux cheatsheets for Omarchy.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    /// Open directly on one sheet (e.g. "nvim", "tmux").
    #[arg(long)]
    pub section: Option<String>,

    /// Print the available sheet names and exit (scriptable).
    #[arg(long)]
    pub list_sections: bool,

    /// Open the search-only window across every sheet, optionally pre-filled.
    #[arg(
        long,
        value_name = "QUERY",
        num_args = 0..=1,
        default_missing_value = "",
        conflicts_with_all = ["section", "list_sections"]
    )]
    pub search: Option<String>,
}

/// Which QML window the GUI opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    /// Sidebar + grouped list of every binding.
    Browser,
    /// Search field + flat ranked hits across every sheet.
    QuickSearch,
}

impl Cli {
    /// The window these arguments ask for.
    pub fn window(&self) -> Window {
        if self.search.is_some() {
            Window::QuickSearch
        } else {
            Window::Browser
        }
    }
}

impl Window {
    /// Source path relative to the crate root, as registered in `build.rs`.
    pub fn qml_path(self) -> &'static str {
        match self {
            Self::Browser => "qml/Main.qml",
            Self::QuickSearch => "qml/QuickSearch.qml",
        }
    }

    /// Qt resource URL the QML module serves this window from.
    pub fn qml_url(self) -> &'static str {
        match self {
            Self::Browser => "qrc:/qt/qml/omarchy/cheatsheets/qml/Main.qml",
            Self::QuickSearch => "qrc:/qt/qml/omarchy/cheatsheets/qml/QuickSearch.qml",
        }
    }
}
