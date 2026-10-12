//! User settings file: defaults, clamping, and round-tripping to disk.

use omarchy_cheatsheets::settings::{self, Settings};

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
