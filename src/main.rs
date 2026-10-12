//! Binary shim: parse args, call library functions, run the GUI.

use anyhow::Context;
use clap::Parser;
use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};
use omarchy_cheatsheets::cli::Cli;
use omarchy_cheatsheets::load;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    if cli.list_sections {
        let mut sheets =
            load::load_all(&load::default_overlay_dir()).context("loading cheatsheets")?;
        load::apply_dynamic_prefixes(&mut sheets, &load::default_config_home());
        let sheets = load::filter_available(sheets, load::command_available);
        for sheet in &sheets {
            println!("{}\t{}", sheet.name, sheet.display_title());
        }
        return Ok(());
    }

    // The QML side picks `--section` up from Qt.application.arguments and
    // calls select_sheet itself, so nothing to thread through here.
    let mut app = QGuiApplication::new();
    if let Some(mut app) = app.as_mut() {
        app.as_mut()
            .set_application_name(&QString::from("omarchy-cheatsheets"));
        app.as_mut()
            .set_application_display_name(&QString::from("Omarchy Cheatsheets"));
    }

    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(&QString::from(
            "qrc:/qt/qml/omarchy/cheatsheets/qml/Main.qml",
        )));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
    Ok(())
}
