//! Take over Omarchy's SUPER+K keybindings menu with this app, on request.
//!
//! Explicitly user-invoked (`--install-bindings`): we never touch Hyprland
//! config from a package scriptlet or without consent.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

/// Errors while editing the Hyprland bindings file.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum InstallError {
    /// The bindings file could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// File that failed.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },
}

/// Convenient result alias.
pub type InstallResult<T> = Result<T, InstallError>;

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingStatus {
    /// The binding line was appended (a timestamped backup was written first).
    Installed,
    /// A SUPER+K binding for this app already exists; nothing changed.
    AlreadyPresent,
}

const BINDING_LINE: &str =
    r#"replace("SUPER + K", "Cheatsheets", { launch = "omarchy-cheatsheets --section hyprland" })"#;

/// User bindings file in Quattro's Lua layout.
pub fn bindings_path(config_home: &Path) -> PathBuf {
    config_home.join("hypr/bindings.lua")
}

/// CLI entry point (`--install-bindings`): install the SUPER+K takeover and
/// print what happened plus the reload instruction.
pub fn run() -> InstallResult<()> {
    let config_home = crate::load::default_config_home();
    match install_binding(&config_home)? {
        BindingStatus::Installed => {
            println!(
                "SUPER + K now opens omarchy-cheatsheets (Hyprland sheet).\n\
                 Backup written next to {}. Run `hyprctl reload` to apply.",
                bindings_path(&config_home).display()
            );
        }
        BindingStatus::AlreadyPresent => {
            println!("SUPER + K already opens omarchy-cheatsheets; nothing to do.");
        }
    }
    Ok(())
}

/// Append the SUPER+K takeover to `bindings.lua`, backing up first.
///
/// Idempotent: any existing `SUPER + K` line mentioning this binary (ours or
/// the user's own edit) counts as installed.
pub fn install_binding(config_home: &Path) -> InstallResult<BindingStatus> {
    let path = bindings_path(config_home);
    let io_err = |source| InstallError::Io {
        path: path.clone(),
        source,
    };
    let source = fs::read_to_string(&path).map_err(io_err)?;
    if source
        .lines()
        .any(|line| line.contains("SUPER + K") && line.contains("omarchy-cheatsheets"))
    {
        return Ok(BindingStatus::AlreadyPresent);
    }

    let backup = path.with_extension(format!("lua.bak.{}", std::process::id()));
    fs::copy(&path, &backup).map_err(io_err)?;

    let mut updated = source;
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str("\n-- omarchy-cheatsheets: take over the stock SUPER+K keybindings menu.\n");
    updated.push_str(BINDING_LINE);
    updated.push('\n');
    fs::write(&path, updated).map_err(io_err)?;
    Ok(BindingStatus::Installed)
}
