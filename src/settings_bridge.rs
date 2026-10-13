//! cxx-qt bridge: exposes appearance settings and the theme palette to QML.
//!
//! Properties are read-only from QML; every change goes through an `apply*`
//! invokable so it is clamped and saved in one place (`settings.rs`).

// cxx-qt's bridge macro generates the unsafe FFI plumbing; nothing here is
// handwritten unsafe. The rest of the crate stays `unsafe_code = "deny"`.
#![allow(unsafe_code)]

use core::pin::Pin;
use std::path::PathBuf;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QVariant};

use crate::settings::{self, Palette, Settings, ThemeRoots};

/// The QObject exposed to QML.
#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// An alias to the QString type
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qlist.h");
        /// An alias to the QVariantList type
        type QList_QVariant = cxx_qt_lib::QList<cxx_qt_lib::QVariant>;

        include!("cxx-qt-lib/qmap.h");
        /// An alias to the QVariantMap type
        type QMap_QString_QVariant = cxx_qt_lib::QMap<cxx_qt_lib::QMapPair_QString_QVariant>;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, theme, READ, NOTIFY)]
        #[qproperty(QString, font_family, READ, NOTIFY)]
        #[qproperty(i32, font_size, READ, NOTIFY)]
        #[qproperty(f64, opacity, READ, NOTIFY)]
        #[qproperty(QString, window_color, READ, NOTIFY)]
        #[qproperty(i32, font_size_min, READ, CONSTANT)]
        #[qproperty(i32, font_size_max, READ, CONSTANT)]
        #[qproperty(i32, font_size_default, READ, CONSTANT)]
        #[qproperty(f64, opacity_min, READ, CONSTANT)]
        type AppSettings = super::AppSettingsRust;

        /// Current palette as a map keyed by QML color property names.
        #[qinvokable]
        fn colors(self: &AppSettings) -> QMap_QString_QVariant;

        /// Theme picker cards: maps with `name`, `label`, `detail` and
        /// `colors` (same keys as `colors()`), "Follow Omarchy" first.
        #[qinvokable]
        #[cxx_name = "themeChoices"]
        fn theme_choices(self: &AppSettings) -> QList_QVariant;

        /// Settings file location for display, with `~` for home.
        #[qinvokable]
        #[cxx_name = "settingsPath"]
        fn settings_path(self: &AppSettings) -> QString;

        /// Switch theme by name ("omarchy" follows the system).
        #[qinvokable]
        #[cxx_name = "applyTheme"]
        fn apply_theme(self: Pin<&mut AppSettings>, name: &QString);

        /// Text font family; empty uses the system default.
        #[qinvokable]
        #[cxx_name = "applyFontFamily"]
        fn apply_font_family(self: Pin<&mut AppSettings>, family: &QString);

        /// Base font size in logical px (clamped).
        #[qinvokable]
        #[cxx_name = "applyFontSize"]
        fn apply_font_size(self: Pin<&mut AppSettings>, size: i32);

        /// Window opacity, 0.5–1.0 (clamped).
        #[qinvokable]
        #[cxx_name = "applyOpacity"]
        fn apply_opacity(self: Pin<&mut AppSettings>, opacity: f64);

        /// Restore every setting to its default.
        #[qinvokable]
        fn reset(self: Pin<&mut AppSettings>);
    }

    impl cxx_qt::Constructor<()> for AppSettings {}
}

impl cxx_qt::Initialize for qobject::AppSettings {
    fn initialize(self: Pin<&mut Self>) {
        let _ = self;
    }
}

/// Rust state behind the settings QObject.
pub struct AppSettingsRust {
    theme: QString,
    font_family: QString,
    font_size: i32,
    opacity: f64,
    window_color: QString,
    font_size_min: i32,
    font_size_max: i32,
    font_size_default: i32,
    opacity_min: f64,
    settings: Settings,
    palette: Palette,
    path: PathBuf,
    roots: ThemeRoots,
}

impl Default for AppSettingsRust {
    fn default() -> Self {
        let path = settings::default_settings_path();
        // A broken file must not keep the window from opening; the next
        // change from the panel rewrites it.
        let loaded = settings::load_settings(&path).unwrap_or_else(|err| {
            tracing::warn!("using default settings: {err}");
            Settings::default()
        });
        let roots = ThemeRoots::system();
        let mut state = Self {
            theme: QString::default(),
            font_family: QString::default(),
            font_size: 0,
            opacity: 0.0,
            window_color: QString::default(),
            font_size_min: i32::try_from(settings::FONT_SIZE_MIN).unwrap_or(0),
            font_size_max: i32::try_from(settings::FONT_SIZE_MAX).unwrap_or(i32::MAX),
            font_size_default: i32::try_from(settings::FONT_SIZE_DEFAULT).unwrap_or(16),
            opacity_min: settings::OPACITY_MIN,
            settings: Settings::default(),
            palette: Palette::builtin(),
            path,
            roots,
        };
        state.adopt(loaded);
        state
    }
}

impl AppSettingsRust {
    /// Mirror `next` into the property fields and recompute the palette.
    fn adopt(&mut self, next: Settings) {
        self.palette = settings::resolve_palette(&next.theme, &self.roots);
        self.theme = QString::from(&next.theme);
        self.font_family = QString::from(&next.font_family);
        self.font_size = i32::try_from(next.font_size).unwrap_or(i32::MAX);
        self.opacity = next.opacity;
        self.window_color = QString::from(&self.palette.window_color(next.opacity));
        self.settings = next;
    }
}

impl qobject::AppSettings {
    /// Current palette as a map keyed by QML color property names.
    pub fn colors(&self) -> QMap<QMapPair_QString_QVariant> {
        palette_map(&self.palette)
    }

    /// Theme picker cards, "Follow Omarchy" first.
    pub fn theme_choices(&self) -> QList<QVariant> {
        let mut list = QList::<QVariant>::default();
        for choice in settings::theme_choices(&self.roots) {
            let mut card = QMap::<QMapPair_QString_QVariant>::default();
            insert_str(&mut card, "name", &choice.name);
            insert_str(&mut card, "label", &choice.label);
            insert_str(&mut card, "detail", choice.detail.as_deref().unwrap_or(""));
            card.insert(
                QString::from("colors"),
                QVariant::from(&palette_map(&choice.palette)),
            );
            list.append(QVariant::from(&card));
        }
        list
    }

    /// Settings file location for display, with `~` for home.
    pub fn settings_path(&self) -> QString {
        let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
        QString::from(&settings::display_path(&self.path, &home))
    }

    /// Switch theme by name ("omarchy" follows the system).
    pub fn apply_theme(self: Pin<&mut Self>, name: &QString) {
        let mut next = self.settings.clone();
        next.theme = name.to_string();
        self.commit(next);
    }

    /// Text font family; empty uses the system default.
    pub fn apply_font_family(self: Pin<&mut Self>, family: &QString) {
        let mut next = self.settings.clone();
        next.font_family = family.to_string();
        self.commit(next);
    }

    /// Base font size in logical px (clamped).
    pub fn apply_font_size(self: Pin<&mut Self>, size: i32) {
        let mut next = self.settings.clone();
        next.font_size = u32::try_from(size).unwrap_or(0);
        self.commit(next);
    }

    /// Window opacity, 0.5–1.0 (clamped).
    pub fn apply_opacity(self: Pin<&mut Self>, opacity: f64) {
        let mut next = self.settings.clone();
        next.opacity = opacity;
        self.commit(next);
    }

    /// Restore every setting to its default.
    pub fn reset(self: Pin<&mut Self>) {
        self.commit(Settings::default());
    }

    /// Clamp, save, and notify QML of whatever actually changed.
    fn commit(mut self: Pin<&mut Self>, next: Settings) {
        let next = next.clamped();
        if next == self.settings {
            return;
        }
        if let Err(err) = settings::save_settings(&self.path, &next) {
            tracing::warn!("settings not saved: {err}");
        }
        let before = self.settings.clone();
        self.as_mut().rust_mut().adopt(next.clone());
        if before.theme != next.theme {
            self.as_mut().theme_changed();
        }
        if before.font_family != next.font_family {
            self.as_mut().font_family_changed();
        }
        if before.font_size != next.font_size {
            self.as_mut().font_size_changed();
        }
        if before.opacity.to_bits() != next.opacity.to_bits() {
            self.as_mut().opacity_changed();
        }
        if before.theme != next.theme || before.opacity.to_bits() != next.opacity.to_bits() {
            self.as_mut().window_color_changed();
        }
    }
}

fn palette_map(palette: &Palette) -> QMap<QMapPair_QString_QVariant> {
    let mut map = QMap::<QMapPair_QString_QVariant>::default();
    for (key, value) in palette.qml_entries() {
        insert_str(&mut map, key, value);
    }
    map
}

fn insert_str(map: &mut QMap<QMapPair_QString_QVariant>, key: &str, value: &str) {
    map.insert(QString::from(key), QVariant::from(&QString::from(value)));
}
