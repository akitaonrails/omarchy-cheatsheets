//! User settings file: defaults, clamping, and round-tripping to disk.

use std::path::Path;

use omarchy_cheatsheets::settings::{self, Palette, Settings, ThemeRoots};

/// Verbatim from /usr/share/omarchy/themes/tokyo-night/colors.toml.
const TOKYO_NIGHT: &str = r##"mode = "dark"

accent = "#7aa2f7"
selection = "#292e42"
muted = "#414868"

background = "#1a1b26"
dark_background = "#13141c"
darker_background = "#0e0e14"
lighter_background = "#24283b"

foreground = "#a9b1d6"
dark_foreground = "#565f89"
light_foreground = "#b4bee6"
bright_foreground = "#c0caf5"

red = "#f7768e"
cyan = "#449dab"
blue = "#7aa2f7"
"##;

const LATTE: &str = r##"mode = "light"
accent = "#1e66f5"
selection = "#ccd0da"
muted = "#9ca0b0"
background = "#eff1f5"
dark_background = "#e6e9ef"
lighter_background = "#dce0e8"
foreground = "#4c4f69"
dark_foreground = "#6c6f85"
bright_foreground = "#4c4f69"
cyan = "#04a5e5"
"##;

fn write_theme(root: &Path, name: &str, colors: &str) {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("colors.toml"), colors).unwrap();
}

#[test]
fn missing_file_gives_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let loaded = settings::load_settings(&dir.path().join("settings.toml")).unwrap();
    assert_eq!(loaded, Settings::default());
}

#[test]
fn defaults_follow_omarchy_and_keep_todays_look() {
    let s = Settings::default();
    assert_eq!(s.theme, settings::FOLLOW_OMARCHY);
    assert_eq!(s.font_family, "");
    assert_eq!(s.font_size, 14);
    assert!((s.opacity - 0.96).abs() < f64::EPSILON);
}

#[test]
fn partial_file_fills_in_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    std::fs::write(&path, "theme = \"nord\"\n").unwrap();

    let loaded = settings::load_settings(&path).unwrap();
    assert_eq!(
        loaded,
        Settings {
            theme: "nord".into(),
            ..Settings::default()
        }
    );
}

#[test]
fn integer_opacity_is_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    std::fs::write(&path, "opacity = 1\n").unwrap();

    let loaded = settings::load_settings(&path).unwrap();
    assert!((loaded.opacity - 1.0).abs() < f64::EPSILON);
}

#[test]
fn out_of_range_values_are_clamped_on_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    std::fs::write(&path, "font_size = 99\nopacity = 0.1\ntheme = \"   \"\n").unwrap();

    let loaded = settings::load_settings(&path).unwrap();
    assert_eq!(loaded.font_size, settings::FONT_SIZE_MAX);
    assert!((loaded.opacity - settings::OPACITY_MIN).abs() < f64::EPSILON);
    assert_eq!(loaded.theme, settings::FOLLOW_OMARCHY);
}

#[test]
fn clamped_raises_tiny_fonts_and_rejects_nan_opacity() {
    let s = Settings {
        font_size: 1,
        opacity: f64::NAN,
        ..Settings::default()
    }
    .clamped();
    assert_eq!(s.font_size, settings::FONT_SIZE_MIN);
    assert!((s.opacity - Settings::default().opacity).abs() < f64::EPSILON);
}

#[test]
fn malformed_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    std::fs::write(&path, "font_size = \"big\"\n").unwrap();
    assert!(settings::load_settings(&path).is_err());
}

#[test]
fn save_then_load_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    let saved = Settings {
        theme: "catppuccin-latte".into(),
        font_family: "Inter".into(),
        font_size: 18,
        opacity: 0.8,
    };
    settings::save_settings(&path, &saved).unwrap();
    assert_eq!(settings::load_settings(&path).unwrap(), saved);
}

#[test]
fn save_creates_the_config_directory() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/omarchy-cheatsheets/settings.toml");
    settings::save_settings(&path, &Settings::default()).unwrap();
    assert!(path.is_file());
}

#[test]
fn save_leaves_no_temp_file_behind() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.toml");
    settings::save_settings(&path, &Settings::default()).unwrap();
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["settings.toml"]);
}

#[test]
fn tokyo_night_maps_to_the_builtin_palette() {
    // Regression: following the stock Omarchy theme must look exactly like
    // the palette Main.qml hardcoded before themes were configurable.
    let palette = Palette::from_colors_toml(TOKYO_NIGHT).unwrap();
    assert_eq!(palette, Palette::builtin());
    assert_eq!(palette.bg, "#1a1b26");
    assert_eq!(palette.bg_dark, "#13141c");
    assert_eq!(palette.surface, "#24283b");
    assert_eq!(palette.selection, "#292e42");
    assert_eq!(palette.muted, "#414868");
    assert_eq!(palette.fg, "#a9b1d6");
    assert_eq!(palette.fg_bright, "#c0caf5");
    assert_eq!(palette.dim, "#565f89");
    assert_eq!(palette.accent, "#7aa2f7");
    assert_eq!(palette.cyan, "#449dab");
}

#[test]
fn missing_or_invalid_colors_fall_back_per_key() {
    let palette =
        Palette::from_colors_toml("accent = \"#ff0000\"\nbackground = \"not-a-color\"\n").unwrap();
    let builtin = Palette::builtin();
    assert_eq!(palette.accent, "#ff0000");
    assert_eq!(palette.bg, builtin.bg);
    assert_eq!(palette.fg, builtin.fg);
}

#[test]
fn unparsable_colors_toml_is_an_error() {
    assert!(Palette::from_colors_toml("accent = ").is_err());
}

#[test]
fn qml_keys_cover_every_color() {
    let palette = Palette::builtin();
    let keys: Vec<_> = palette.qml_entries().iter().map(|(k, _)| *k).collect();
    assert_eq!(
        keys,
        [
            "bg",
            "bgDark",
            "surface",
            "selection",
            "muted",
            "fg",
            "fgBright",
            "dim",
            "accent",
            "cyan"
        ]
    );
}

#[test]
fn available_themes_dedupes_sorts_and_skips_dirs_without_colors() {
    let user = tempfile::tempdir().unwrap();
    let system = tempfile::tempdir().unwrap();
    write_theme(system.path(), "tokyo-night", TOKYO_NIGHT);
    write_theme(system.path(), "catppuccin-latte", LATTE);
    write_theme(user.path(), "tokyo-night", LATTE);
    std::fs::create_dir_all(user.path().join("aether")).unwrap();

    let roots = ThemeRoots {
        current: user.path().join("missing"),
        dirs: vec![user.path().to_path_buf(), system.path().to_path_buf()],
    };
    assert_eq!(
        settings::available_themes(&roots),
        ["catppuccin-latte", "tokyo-night"]
    );
}

#[test]
fn follow_omarchy_reads_the_current_theme() {
    let state = tempfile::tempdir().unwrap();
    std::fs::write(state.path().join("colors.toml"), LATTE).unwrap();
    let roots = ThemeRoots {
        current: state.path().to_path_buf(),
        dirs: vec![],
    };
    let palette = settings::resolve_palette(settings::FOLLOW_OMARCHY, &roots);
    assert_eq!(palette.bg, "#eff1f5");
}

#[test]
fn named_theme_prefers_the_user_root() {
    let user = tempfile::tempdir().unwrap();
    let system = tempfile::tempdir().unwrap();
    write_theme(user.path(), "nord", LATTE);
    write_theme(system.path(), "nord", TOKYO_NIGHT);
    let roots = ThemeRoots {
        current: user.path().join("missing"),
        dirs: vec![user.path().to_path_buf(), system.path().to_path_buf()],
    };
    assert_eq!(settings::resolve_palette("nord", &roots).bg, "#eff1f5");
}

#[test]
fn unknown_theme_falls_back_to_current_then_builtin() {
    let state = tempfile::tempdir().unwrap();
    std::fs::write(state.path().join("colors.toml"), LATTE).unwrap();
    let with_current = ThemeRoots {
        current: state.path().to_path_buf(),
        dirs: vec![],
    };
    assert_eq!(
        settings::resolve_palette("no-such-theme", &with_current).bg,
        "#eff1f5"
    );

    let nothing = ThemeRoots {
        current: state.path().join("missing"),
        dirs: vec![],
    };
    assert_eq!(
        settings::resolve_palette("no-such-theme", &nothing),
        Palette::builtin()
    );
}

#[test]
fn theme_names_cannot_escape_the_theme_roots() {
    let root = tempfile::tempdir().unwrap();
    let themes = root.path().join("themes");
    std::fs::create_dir_all(&themes).unwrap();
    write_theme(root.path(), "outside", LATTE);
    let roots = ThemeRoots {
        current: root.path().join("missing"),
        dirs: vec![themes],
    };
    assert_eq!(
        settings::resolve_palette("../outside", &roots),
        Palette::builtin()
    );
}
