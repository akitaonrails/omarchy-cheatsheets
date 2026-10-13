//! Loading and merging cheatsheets: embedded TOML plus user overlays.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::sheet::Sheet;

/// Errors while reading or parsing cheatsheet data.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LoadError {
    /// A TOML file failed to parse.
    #[error("failed to parse {path}: {source}")]
    Parse {
        /// File (or embedded resource) that failed.
        path: String,
        /// Underlying TOML error.
        source: toml::de::Error,
    },
    /// An overlay file could not be read.
    #[error("failed to read {path}: {source}")]
    Io {
        /// File that failed.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
}

/// Convenient result alias for loading operations.
pub type LoadResult<T> = Result<T, LoadError>;

/// Appearance settings file; shares the overlay directory but is not a sheet.
pub const SETTINGS_FILE: &str = "settings.toml";

const EMBEDDED: &[(&str, &str)] = &[
    ("nvim", include_str!("../data/nvim.toml")),
    ("tmux", include_str!("../data/tmux.toml")),
    ("herdr", include_str!("../data/herdr.toml")),
    ("zellij", include_str!("../data/zellij.toml")),
    ("kitty", include_str!("../data/kitty.toml")),
    ("alacritty", include_str!("../data/alacritty.toml")),
    ("ghostty", include_str!("../data/ghostty.toml")),
    ("hyprland", include_str!("../data/hyprland.toml")),
    ("lazygit", include_str!("../data/lazygit.toml")),
    ("lazydocker", include_str!("../data/lazydocker.toml")),
    ("btop", include_str!("../data/btop.toml")),
];

/// Whether `command` resolves to an executable file in PATH.
pub fn command_available(command: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| {
        let candidate = dir.join(command);
        candidate.is_file() && is_executable(&candidate)
    })
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    true
}

/// Drop sheets whose `command` is not available on this system.
/// Sheets without a command are always kept.
pub fn filter_available(sheets: Vec<Sheet>, available: impl Fn(&str) -> bool) -> Vec<Sheet> {
    sheets
        .into_iter()
        .filter(|s| s.command.as_deref().is_none_or(&available))
        .collect()
}

/// Parse one TOML document into a [`Sheet`].
pub fn parse_sheet(source: &str, path: &str) -> LoadResult<Sheet> {
    toml::from_str(source).map_err(|source| LoadError::Parse {
        path: path.to_string(),
        source,
    })
}

/// Directory where user overlay sheets live (`~/.config/omarchy-cheatsheets`).
pub fn default_overlay_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return PathBuf::from(xdg).join("omarchy-cheatsheets");
    }
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".config/omarchy-cheatsheets")
}

/// Directory holding user configs (`$XDG_CONFIG_HOME` or `~/.config`).
pub fn default_config_home() -> PathBuf {
    default_overlay_dir()
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// Load the embedded sheets with overlays from `overlay_dir` merged on top.
///
/// A missing or unreadable overlay directory is not an error — embedded
/// sheets always work. A malformed overlay *file* is an error: silent
/// fallback would hide typos the user meant to apply.
pub fn load_all(overlay_dir: &Path) -> LoadResult<Vec<Sheet>> {
    let mut sheets = EMBEDDED
        .iter()
        .map(|(name, source)| parse_sheet(source, &format!("embedded:{name}.toml")))
        .collect::<LoadResult<Vec<_>>>()?;

    let Ok(read_dir) = fs::read_dir(overlay_dir) else {
        return Ok(sheets);
    };
    let mut overlays: Vec<PathBuf> = read_dir
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
        .filter(|p| p.file_name().is_none_or(|name| name != SETTINGS_FILE))
        .collect();
    overlays.sort();

    for path in overlays {
        let source = fs::read_to_string(&path).map_err(|source| LoadError::Io {
            path: path.clone(),
            source,
        })?;
        let overlay = parse_sheet(&source, &path.display().to_string())?;
        merge_overlay(&mut sheets, overlay);
    }
    Ok(sheets)
}

/// Merge one overlay sheet into the loaded set.
///
/// Same-name sheets merge section by section (same-name sections merge entry
/// by entry, replacing on identical key chords); anything new is appended,
/// so an overlay can both patch existing sheets and add whole new programs.
fn merge_overlay(sheets: &mut Vec<Sheet>, overlay: Sheet) {
    let Some(base) = sheets.iter_mut().find(|s| s.name == overlay.name) else {
        sheets.push(overlay);
        return;
    };
    for section in overlay.sections {
        let Some(base_section) = base.sections.iter_mut().find(|s| s.name == section.name) else {
            base.sections.push(section);
            continue;
        };
        for entry in section.entries {
            let key = entry.merge_key();
            match base_section
                .entries
                .iter_mut()
                .find(|e| e.merge_key() == key)
            {
                Some(existing) => *existing = entry,
                None => base_section.entries.push(entry),
            }
        }
    }
}
