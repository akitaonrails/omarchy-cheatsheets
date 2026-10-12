//! cxx-qt bridge: exposes loaded sheets and live filtering to QML.
//!
//! QML stays thin — it renders `filtered_rows()` and re-queries it when
//! `searchText`/`currentSheet` change (their NOTIFY signals drive the
//! refresh; no custom model class is needed at this corpus size).

// cxx-qt's bridge macro generates the unsafe FFI plumbing; nothing here is
// handwritten unsafe. The rest of the crate stays `unsafe_code = "deny"`.
#![allow(unsafe_code)]

use core::pin::Pin;

use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QStringList, QVariant};

use crate::load;
use crate::resolve;
use crate::search;
use crate::sheet::Sheet;

/// The QObject exposed to QML.
#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        /// An alias to the QString type
        type QString = cxx_qt_lib::QString;

        include!("cxx-qt-lib/qstringlist.h");
        /// An alias to the QStringList type
        type QStringList = cxx_qt_lib::QStringList;

        include!("cxx-qt-lib/qvariant.h");
        /// An alias to the QVariant type
        type QVariant = cxx_qt_lib::QVariant;

        include!("cxx-qt-lib/qlist.h");
        /// An alias to the QVariantList type
        type QList_QVariant = cxx_qt_lib::QList<cxx_qt_lib::QVariant>;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, search_text)]
        #[qproperty(i32, current_sheet)]
        type CheatsheetBridge = super::CheatsheetBridgeRust;

        /// Names of all loaded sheets, for the sidebar.
        #[qinvokable]
        #[cxx_name = "sheetNames"]
        fn sheet_names(self: &CheatsheetBridge) -> QStringList;

        /// Display titles of all loaded sheets, parallel to `sheet_names`.
        #[qinvokable]
        #[cxx_name = "sheetTitles"]
        fn sheet_titles(self: &CheatsheetBridge) -> QStringList;

        /// Sidebar icons (Nerd Font glyphs), parallel to `sheet_names`.
        #[qinvokable]
        #[cxx_name = "sheetIcons"]
        fn sheet_icons(self: &CheatsheetBridge) -> QStringList;

        /// Rows matching the current query/sheet as a list of maps with
        /// `sheet`, `section`, `keys` (string list) and `description`.
        #[qinvokable]
        #[cxx_name = "filteredRows"]
        fn filtered_rows(self: &CheatsheetBridge) -> QList_QVariant;

        /// Select a sheet by name; unknown or empty selects "all".
        #[qinvokable]
        #[cxx_name = "selectSheet"]
        fn select_sheet(self: Pin<&mut CheatsheetBridge>, name: &QString);
    }

    impl cxx_qt::Constructor<()> for CheatsheetBridge {}
}

// QML instantiates the bridge, so it needs a default constructor; loading
// happens in the Rust struct's Default impl below.
impl cxx_qt::Initialize for qobject::CheatsheetBridge {
    fn initialize(self: Pin<&mut Self>) {
        let _ = self;
    }
}

/// Rust state behind the bridge QObject.
pub struct CheatsheetBridgeRust {
    search_text: QString,
    /// Index into `sheets`; -1 means "all sheets".
    current_sheet: i32,
    sheets: Vec<Sheet>,
}

impl Default for CheatsheetBridgeRust {
    fn default() -> Self {
        let mut sheets = match load::load_all(&load::default_overlay_dir()) {
            Ok(sheets) => sheets,
            Err(err) => {
                tracing::warn!("failed to load overlays, using embedded sheets: {err}");
                load::load_all(std::path::Path::new("/nonexistent")).unwrap_or_default()
            }
        };
        let config_home = load::default_config_home();
        load::apply_dynamic_prefixes(&mut sheets, &config_home);
        resolve::apply_resolved_bindings(&mut sheets, &config_home);
        let sheets = load::filter_available(sheets, load::command_available);
        Self {
            search_text: QString::default(),
            current_sheet: -1,
            sheets,
        }
    }
}

impl qobject::CheatsheetBridge {
    /// Names of all loaded sheets, for the sidebar.
    pub fn sheet_names(&self) -> QStringList {
        self.sheets.iter().map(|s| QString::from(&s.name)).collect()
    }

    /// Display titles of all loaded sheets, parallel to `sheet_names`.
    pub fn sheet_titles(&self) -> QStringList {
        self.sheets
            .iter()
            .map(|s| QString::from(s.display_title()))
            .collect()
    }

    /// Sidebar icons (Nerd Font glyphs), parallel to `sheet_names`.
    pub fn sheet_icons(&self) -> QStringList {
        self.sheets
            .iter()
            .map(|s| QString::from(s.icon.as_deref().unwrap_or("\u{f02d}")))
            .collect()
    }

    /// Rows matching the current query/sheet as QVariantMaps.
    pub fn filtered_rows(&self) -> QList<QVariant> {
        let query = self.search_text.to_string();
        let sheets: &[Sheet] = match usize::try_from(self.current_sheet) {
            Ok(i) => self.sheets.get(i..).map_or(&[][..], |rest| &rest[..1]),
            Err(_) => &self.sheets,
        };

        let mut rows = QList::<QVariant>::default();
        for section in search::filter(sheets, &query) {
            // Scoped view groups by section; the all-sheets view prefixes the
            // app title so every row is attributable ("tmux · Panes").
            let group = if self.current_sheet < 0 {
                format!("{} · {}", section.sheet_title, section.name)
            } else {
                section.name.clone()
            };
            for scored in section.entries {
                let keys: QStringList = scored.entry.keys.iter().map(QString::from).collect();
                let mut row = QMap::<QMapPair_QString_QVariant>::default();
                row.insert(
                    QString::from("sheet"),
                    QVariant::from(&QString::from(&section.sheet)),
                );
                row.insert(
                    QString::from("section"),
                    QVariant::from(&QString::from(&section.name)),
                );
                row.insert(
                    QString::from("group"),
                    QVariant::from(&QString::from(&group)),
                );
                row.insert(QString::from("keys"), QVariant::from(&keys));
                row.insert(
                    QString::from("description"),
                    QVariant::from(&QString::from(&scored.entry.description)),
                );
                rows.append(QVariant::from(&row));
            }
        }
        rows
    }

    /// Select a sheet by name; unknown or empty selects "all".
    pub fn select_sheet(self: Pin<&mut Self>, name: &QString) {
        let name = name.to_string();
        let index = self
            .sheets
            .iter()
            .position(|s| s.name == name)
            .map_or(-1, |i| i32::try_from(i).unwrap_or(-1));
        self.set_current_sheet(index);
    }
}
