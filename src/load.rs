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

const EMBEDDED: &[(&str, &str)] = &[
    ("nvim", include_str!("../data/nvim.toml")),
    ("tmux", include_str!("../data/tmux.toml")),
    ("herdr", include_str!("../data/herdr.toml")),
    ("zellij", include_str!("../data/zellij.toml")),
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

/// Substitute `{prefix}` in sheet titles with the key resolved from the
/// user's actual config files. Sheets whose tool has no discoverable
/// prefix fall back to their static title (template stripped).
pub fn apply_dynamic_prefixes(sheets: &mut [Sheet], config_home: &Path) {
    for sheet in sheets.iter_mut() {
        let resolved = resolve_prefix(&sheet.name, config_home);
        let title = sheet.title.take().unwrap_or_else(|| sheet.name.clone());
        sheet.title = Some(render_title(&title, resolved.as_deref()));
    }
}

fn render_title(template: &str, prefix: Option<&str>) -> String {
    match prefix {
        Some(p) => template.replace("{prefix}", p),
        None => template
            .replace(" (prefix: {prefix})", "")
            .replace(", leader: {prefix})", ")")
            .replace("{prefix}", "prefix"),
    }
}

/// Resolve a tool's prefix/leader from the user's config. Returns a display
/// string like `Ctrl+Space / Ctrl+b`.
fn resolve_prefix(sheet: &str, config_home: &Path) -> Option<String> {
    match sheet {
        "tmux" => resolve_tmux_prefix(config_home),
        "herdr" => resolve_herdr_prefix(config_home),
        "nvim" => resolve_nvim_leader(config_home),
        _ => None,
    }
}

/// tmux chord notation (`C-Space`, `M-Enter`) to display form.
fn tmux_chord(chord: &str) -> String {
    crate::resolve::display_chord(chord).join("+")
}

fn resolve_tmux_prefix(config_home: &Path) -> Option<String> {
    let conf = config_home.join("tmux/tmux.conf");
    let conf = if conf.exists() {
        conf
    } else {
        // Pre-XDG location
        let home = std::env::var("HOME").ok()?;
        PathBuf::from(home).join(".tmux.conf")
    };
    let source = fs::read_to_string(conf).ok()?;

    let mut prefixes = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("set -g prefix ") {
            prefixes.push(tmux_chord(rest.trim()));
        } else if let Some(rest) = line.strip_prefix("set -g prefix2 ") {
            prefixes.push(tmux_chord(rest.trim()));
        }
    }
    (!prefixes.is_empty()).then(|| prefixes.join(" / "))
}

fn resolve_herdr_prefix(config_home: &Path) -> Option<String> {
    let conf = config_home.join("herdr/config.toml");
    let source = fs::read_to_string(conf).ok()?;
    let doc: toml::Value = toml::from_str(&source).ok()?;
    let prefix = doc.get("keys")?.get("prefix")?.as_str()?;
    Some(crate::resolve::display_chord(prefix).join("+"))
}

/// LazyVim reads `vim.g.mapleader`; default is Space. Scan the user's lua
/// config for an override.
fn resolve_nvim_leader(config_home: &Path) -> Option<String> {
    let lua_dir = config_home.join("nvim/lua");
    let source = read_lua_sources(&lua_dir)?;
    for line in source.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("vim.g.mapleader") {
            let value = rest
                .trim_start_matches(['=', ' ', '"'])
                .trim_end_matches(['"', ' ']);
            return Some(match value {
                " " | "" => "Space".to_string(),
                other => other.to_string(),
            });
        }
    }
    Some("Space".to_string())
}

fn read_lua_sources(dir: &Path) -> Option<String> {
    let mut out = String::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "lua") {
                out.push_str(&fs::read_to_string(path).unwrap_or_default());
                out.push('\n');
            }
        }
    }
    (!out.is_empty()).then_some(out)
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
