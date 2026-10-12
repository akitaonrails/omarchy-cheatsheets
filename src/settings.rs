//! User appearance settings (`settings.toml`): theme, font, size, opacity.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::load;

/// `theme` value that follows the active Omarchy theme.
pub const FOLLOW_OMARCHY: &str = "omarchy";
/// Smallest base font size (logical px).
pub const FONT_SIZE_MIN: u32 = 10;
/// Largest base font size (logical px).
pub const FONT_SIZE_MAX: u32 = 24;
/// Base font size the QML layout was designed at.
pub const FONT_SIZE_DEFAULT: u32 = 14;
/// Lowest window opacity; below this the list is unreadable over wallpapers.
pub const OPACITY_MIN: f64 = 0.5;
/// Fully opaque window.
pub const OPACITY_MAX: f64 = 1.0;
/// Slight translucency so Hyprland's blur bleeds through.
pub const OPACITY_DEFAULT: f64 = 0.96;

/// Errors while reading or writing the settings file.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SettingsError {
    /// The file exists but could not be read or written.
    #[error("settings file {path}: {source}")]
    Io {
        /// File that failed.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
    /// The file is not valid settings TOML.
    #[error("failed to parse {path}: {source}")]
    Parse {
        /// File that failed.
        path: PathBuf,
        /// Underlying TOML error.
        source: toml::de::Error,
    },
    /// Settings could not be encoded as TOML.
    #[error("failed to encode settings: {0}")]
    Encode(#[from] toml::ser::Error),
}

/// Appearance choices persisted between runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Omarchy theme name, or [`FOLLOW_OMARCHY`] for the active one.
    pub theme: String,
    /// Text font family; empty uses the system default.
    pub font_family: String,
    /// Base font size in logical px; the layout scales from it.
    pub font_size: u32,
    /// Window background opacity, 0.5–1.0.
    pub opacity: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: FOLLOW_OMARCHY.to_string(),
            font_family: String::new(),
            font_size: FONT_SIZE_DEFAULT,
            opacity: OPACITY_DEFAULT,
        }
    }
}

impl Settings {
    /// Pull every field back into its supported range.
    #[must_use]
    pub fn clamped(mut self) -> Self {
        self.theme = self.theme.trim().to_string();
        if self.theme.is_empty() {
            self.theme = FOLLOW_OMARCHY.to_string();
        }
        self.font_family = self.font_family.trim().to_string();
        self.font_size = self.font_size.clamp(FONT_SIZE_MIN, FONT_SIZE_MAX);
        self.opacity = if self.opacity.is_finite() {
            self.opacity.clamp(OPACITY_MIN, OPACITY_MAX)
        } else {
            OPACITY_DEFAULT
        };
        self
    }
}

/// Where the settings live: next to the overlays, under a reserved name.
pub fn default_settings_path() -> PathBuf {
    load::default_overlay_dir().join(load::SETTINGS_FILE)
}

/// Read settings from `path`; a missing file means defaults.
pub fn load_settings(path: &Path) -> Result<Settings, SettingsError> {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Settings::default());
        }
        Err(source) => {
            return Err(SettingsError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let settings: Settings = toml::from_str(&source).map_err(|source| SettingsError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(settings.clamped())
}

/// Write settings to `path`, creating its directory.
///
/// Writes a sibling temp file and renames it over the target, so a crash
/// mid-write never leaves a truncated file that would reset every choice.
pub fn save_settings(path: &Path, settings: &Settings) -> Result<(), SettingsError> {
    let io_err = |source| SettingsError::Io {
        path: path.to_path_buf(),
        source,
    };
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(io_err)?;
    }
    let body = toml::to_string(settings)?;
    let tmp = path.with_extension("toml.tmp");
    fs::write(&tmp, body).map_err(io_err)?;
    fs::rename(&tmp, path).map_err(io_err)
}
