//! Cheatsheet data model: sheets contain sections, sections contain entries.

use serde::Deserialize;

/// One cheatsheet for a program (e.g. Neovim, tmux).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Sheet {
    /// Display name and merge key, e.g. "nvim".
    pub name: String,
    /// Longer human title shown in the UI header, e.g. "Neovim (LazyVim)".
    #[serde(default)]
    pub title: Option<String>,
    /// Nerd Font glyph for the sidebar, e.g. "\u{e795}".
    #[serde(default)]
    pub icon: Option<String>,
    /// Executable probed in PATH; sheets for missing tools are hidden.
    #[serde(default)]
    pub command: Option<String>,
    /// Grouped bindings, in relevancy order.
    #[serde(default)]
    pub sections: Vec<Section>,
}

/// A named group of related bindings inside a sheet.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Section {
    /// Group name, e.g. "Motions".
    pub name: String,
    /// Bindings in this group, in display order.
    #[serde(default)]
    pub entries: Vec<Entry>,
}

/// A single keybinding and what it does.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Entry {
    /// Key chords to render as keycaps, e.g. `["Ctrl", "Space"]` then `["c"]`.
    pub keys: Vec<String>,
    /// What the binding does.
    pub description: String,
    /// Free-form labels ("daily", "rare", ...) reserved for relevancy sorting.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Sheet {
    /// Human-facing title, falling back to the sheet name.
    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or(&self.name)
    }
}

impl Entry {
    /// Stable identity used when merging overlay entries: same keys, same entry.
    pub fn merge_key(&self) -> String {
        self.keys.join("+")
    }

    /// Flat lowercase haystack for search: keys + description + tags.
    pub fn haystack(&self) -> String {
        format!(
            "{} {} {}",
            self.keys.join(" "),
            self.description,
            self.tags.join(" ")
        )
    }
}
