//! SUPER+K takeover: idempotent, backs up, never clobbers.

use omarchy_cheatsheets::install::{self, BindingStatus};

fn config_home_with_bindings(body: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("hypr")).unwrap();
    std::fs::write(tmp.path().join("hypr/bindings.lua"), body).unwrap();
    tmp
}

#[test]
fn installs_with_backup_and_is_idempotent() {
    let tmp = config_home_with_bindings("o.bind(\"SUPER + RETURN\", \"Terminal\", \"x\")\n");
    let path = install::bindings_path(tmp.path());

    assert_eq!(
        install::install_binding(tmp.path()).unwrap(),
        BindingStatus::Installed
    );
    let content = std::fs::read_to_string(&path).unwrap();
    assert!(content.contains("SUPER + K"));
    assert!(content.contains("omarchy-cheatsheets --section hyprland"));
    // Regression: Lua comments are `--`, not `#` (a `#` comment once broke
    // `hyprctl reload` with "unexpected symbol").
    assert!(content.contains("\n-- omarchy-cheatsheets:"));
    assert!(!content.lines().any(|l| l.starts_with('#')));
    assert!(content.contains("SUPER + RETURN"), "existing bindings kept");
    assert!(
        std::fs::read_dir(tmp.path().join("hypr"))
            .unwrap()
            .any(|e| e.unwrap().file_name().to_string_lossy().contains(".bak."))
    );

    assert_eq!(
        install::install_binding(tmp.path()).unwrap(),
        BindingStatus::AlreadyPresent,
        "second run is a no-op"
    );
    let count = std::fs::read_to_string(&path)
        .unwrap()
        .matches("omarchy-cheatsheets --section hyprland")
        .count();
    assert_eq!(count, 1);
}

#[test]
fn respects_users_own_super_k_takeover() {
    let tmp = config_home_with_bindings(
        "replace(\"SUPER + K\", \"Cheatsheets\", { launch = \"omarchy-cheatsheets\" })\n",
    );
    assert_eq!(
        install::install_binding(tmp.path()).unwrap(),
        BindingStatus::AlreadyPresent
    );
}
