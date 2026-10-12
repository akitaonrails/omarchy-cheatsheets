//! Build script: cxx-qt C++ codegen, QML module registration, Qt linking.

use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(QmlModule::new("omarchy.cheatsheets").qml_file("qml/Main.qml"))
        .qt_module("Gui")
        .qt_module("Qml")
        .files(["src/bridge.rs", "src/settings_bridge.rs"])
        .build();
}
