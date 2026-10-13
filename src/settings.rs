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

/// Palette file every Omarchy theme directory ships.
const COLORS_FILE: &str = "colors.toml";

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

/// Window colors, as `#rrggbb` strings QML converts to `color`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    /// Window background (`background`).
    pub bg: String,
    /// Keycap gradient bottom (`dark_background`).
    pub bg_dark: String,
    /// Search field, keycaps, dividers (`lighter_background`).
    pub surface: String,
    /// Selected/hovered row (`selection`).
    pub selection: String,
    /// Borders, empty-state icon (`muted`).
    pub muted: String,
    /// Body text (`foreground`).
    pub fg: String,
    /// Emphasized text, keycap labels (`bright_foreground`).
    pub fg_bright: String,
    /// Hints, placeholders, counters (`dark_foreground`).
    pub dim: String,
    /// Focus ring, section headers, active sidebar item (`accent`).
    pub accent: String,
    /// Secondary accent (`cyan`).
    pub cyan: String,
}

impl Palette {
    /// Tokyo Night, the stock Omarchy theme and the look before theming.
    pub fn builtin() -> Self {
        Self {
            bg: "#1a1b26".into(),
            bg_dark: "#13141c".into(),
            surface: "#24283b".into(),
            selection: "#292e42".into(),
            muted: "#414868".into(),
            fg: "#a9b1d6".into(),
            fg_bright: "#c0caf5".into(),
            dim: "#565f89".into(),
            accent: "#7aa2f7".into(),
            cyan: "#449dab".into(),
        }
    }

    /// Map an Omarchy theme `colors.toml`; missing or malformed colors keep
    /// the builtin value so a partial custom theme still renders.
    pub fn from_colors_toml(source: &str) -> Result<Self, toml::de::Error> {
        let table: toml::Table = toml::from_str(source)?;
        let pick = |key: &str, fallback: String| {
            table
                .get(key)
                .and_then(toml::Value::as_str)
                .filter(|v| is_hex_color(v))
                .map_or(fallback, str::to_string)
        };
        let b = Self::builtin();
        Ok(Self {
            bg: pick("background", b.bg),
            bg_dark: pick("dark_background", b.bg_dark),
            surface: pick("lighter_background", b.surface),
            selection: pick("selection", b.selection),
            muted: pick("muted", b.muted),
            fg: pick("foreground", b.fg),
            fg_bright: pick("bright_foreground", b.fg_bright),
            dim: pick("dark_foreground", b.dim),
            accent: pick("accent", b.accent),
            cyan: pick("cyan", b.cyan),
        })
    }

    /// Colors keyed by the property names Main.qml binds to.
    pub fn qml_entries(&self) -> [(&'static str, &str); 10] {
        [
            ("bg", &self.bg),
            ("bgDark", &self.bg_dark),
            ("surface", &self.surface),
            ("selection", &self.selection),
            ("muted", &self.muted),
            ("fg", &self.fg),
            ("fgBright", &self.fg_bright),
            ("dim", &self.dim),
            ("accent", &self.accent),
            ("cyan", &self.cyan),
        ]
    }

    /// Window background at `opacity` as QML `#AARRGGBB`, premultiplied:
    /// Qt Quick hands the window color to the compositor as premultiplied
    /// alpha, so straight RGB adds light instead of letting the desktop show
    /// through (a light theme turns solid white). NaN counts as opaque.
    pub fn window_color(&self, opacity: f64) -> String {
        let opacity = if opacity.is_nan() {
            1.0
        } else {
            opacity.clamp(0.0, 1.0)
        };
        // In 0..=255 after the clamp, so the casts cannot truncate.
        let alpha = (opacity * 255.0).round() as u8;
        let channel = |i: usize| {
            let straight = self
                .bg
                .get(i..i + 2)
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
                .unwrap_or(0);
            (f64::from(straight) * f64::from(alpha) / 255.0).round() as u8
        };
        format!(
            "#{alpha:02x}{:02x}{:02x}{:02x}",
            channel(1),
            channel(3),
            channel(5)
        )
    }
}

fn is_hex_color(value: &str) -> bool {
    value
        .strip_prefix('#')
        .is_some_and(|hex| hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Where Omarchy themes live; injected so tests never read the real system.
#[derive(Debug, Clone)]
pub struct ThemeRoots {
    /// The active theme directory (holds `colors.toml`).
    pub current: PathBuf,
    /// Theme collections searched in order; earlier roots win.
    pub dirs: Vec<PathBuf>,
}

impl ThemeRoots {
    /// Omarchy's real locations: active theme state, user themes, stock themes.
    pub fn system() -> Self {
        let state_home = std::env::var_os("XDG_STATE_HOME")
            .filter(|v| !v.is_empty())
            .map_or_else(
                || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"),
                PathBuf::from,
            );
        Self {
            current: state_home.join("omarchy/current/theme"),
            dirs: vec![
                load::default_config_home().join("omarchy/themes"),
                PathBuf::from("/usr/share/omarchy/themes"),
            ],
        }
    }
}

/// Theme names found under `roots.dirs` that ship a `colors.toml`, sorted.
pub fn available_themes(roots: &ThemeRoots) -> Vec<String> {
    let mut names: Vec<String> = roots
        .dirs
        .iter()
        .filter_map(|dir| fs::read_dir(dir).ok())
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| e.path().join(COLORS_FILE).is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Palette for `theme`: the named theme, else the active Omarchy theme,
/// else the builtin — the window always gets colors.
pub fn resolve_palette(theme: &str, roots: &ThemeRoots) -> Palette {
    let named = (theme != FOLLOW_OMARCHY && is_plain_name(theme))
        .then(|| roots.dirs.iter().map(|dir| dir.join(theme)))
        .into_iter()
        .flatten();
    named
        .chain(std::iter::once(roots.current.clone()))
        .find_map(|dir| read_palette(&dir))
        .unwrap_or_else(Palette::builtin)
}

/// A theme name is one path component; anything else could escape the roots.
fn is_plain_name(name: &str) -> bool {
    let mut parts = Path::new(name).components();
    matches!(
        (parts.next(), parts.next()),
        (Some(std::path::Component::Normal(_)), None)
    )
}

fn read_palette(theme_dir: &Path) -> Option<Palette> {
    let source = fs::read_to_string(theme_dir.join(COLORS_FILE)).ok()?;
    match Palette::from_colors_toml(&source) {
        Ok(palette) => Some(palette),
        Err(err) => {
            tracing::warn!("ignoring theme {}: {err}", theme_dir.display());
            None
        }
    }
}

/// One card in the theme picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeChoice {
    /// Value stored in `settings.toml` (`theme`).
    pub name: String,
    /// Human name, e.g. "Catppuccin Latte".
    pub label: String,
    /// Extra line under the label: the active theme for "Follow Omarchy".
    pub detail: Option<String>,
    /// Colors the card previews.
    pub palette: Palette,
}

/// Title-case a theme directory name ("tokyo-night" → "Tokyo Night"), the
/// same rule `omarchy theme current` uses.
pub fn theme_label(name: &str) -> String {
    name.split('-')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// "Follow Omarchy" first, then every installed theme with its own palette.
pub fn theme_choices(roots: &ThemeRoots) -> Vec<ThemeChoice> {
    let active = fs::read_to_string(roots.current.with_file_name("theme.name"))
        .ok()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty());
    let follow = ThemeChoice {
        name: FOLLOW_OMARCHY.to_string(),
        label: "Follow Omarchy".to_string(),
        detail: active.as_deref().map(theme_label),
        palette: read_palette(&roots.current).unwrap_or_else(Palette::builtin),
    };
    let installed = available_themes(roots).into_iter().map(|name| ThemeChoice {
        label: theme_label(&name),
        detail: None,
        palette: resolve_palette(&name, roots),
        name,
    });
    std::iter::once(follow).chain(installed).collect()
}

/// `path` with the home directory shown as `~`, for the panel footer.
pub fn display_path(path: &Path, home: &Path) -> String {
    if home.is_absolute()
        && let Ok(rest) = path.strip_prefix(home)
    {
        return Path::new("~").join(rest).display().to_string();
    }
    path.display().to_string()
}
