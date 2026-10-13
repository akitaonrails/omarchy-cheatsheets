//! Live-resolution parsers: tmux list-keys and herdr [keys] merging.

use omarchy_cheatsheets::resolve::{display_chord, parse_herdr_keys, parse_tmux_list_keys};

#[test]
fn parses_tmux_list_keys_tables() {
    let output = "\
bind-key -T prefix h split-window -v -c \"#{pane_current_path}\"
bind-key -T root C-M-Left select-pane -L
bind-key -n M-Enter split-window -v
bind-key -T copy-mode-vi v send-keys -X begin-selection
";
    let keys = parse_tmux_list_keys(output);
    assert_eq!(keys.len(), 4);
    assert_eq!(keys[0].table, "prefix");
    assert_eq!(keys[0].chord, "h");
    assert!(keys[0].command.contains("split-window -v"));
    assert_eq!(keys[1].table, "root");
    assert_eq!(keys[2].table, "root", "-n means root table");
    assert_eq!(keys[3].table, "copy-mode-vi");
}

#[test]
fn chord_display_normalizes_tmux_and_herdr_notation() {
    assert_eq!(display_chord("C-Space"), ["Ctrl", "Space"]);
    assert_eq!(display_chord("C-M-Left"), ["Ctrl", "Alt", "←"]);
    assert_eq!(display_chord("ctrl+shift+o"), ["Ctrl", "Shift", "o"]);
    assert_eq!(display_chord("prefix+1..9"), ["prefix", "1..9"]);
    // Regression: single-letter keys in final position are keys, not
    // modifiers — "ctrl+shift+c" must not render as "Ctrl Shift Ctrl".
    assert_eq!(display_chord("ctrl+shift+c"), ["Ctrl", "Shift", "c"]);
    assert_eq!(display_chord("ctrl+shift+s"), ["Ctrl", "Shift", "s"]);
    assert_eq!(display_chord("ctrl+shift+m"), ["Ctrl", "Shift", "m"]);
}

#[test]
fn herdr_commented_defaults_plus_user_overrides() {
    let defaults = "\
# [theme]
# name = \"terminal\"
[keys]
# prefix = \"ctrl+space\"
# split_horizontal = [\"prefix+h\", \"alt+enter\"]
# zoom = \"prefix+z\"
[ui]
# accent = \"blue\"
";
    let user = "[keys]\nprefix = \"ctrl+b\"\nzoom = \"\"\n";
    let map = parse_herdr_keys(defaults, Some(user));
    assert_eq!(map["prefix"], vec!["ctrl+b"]);
    assert_eq!(map["split_horizontal"], vec!["prefix+h", "alt+enter"]);
    assert!(map["zoom"].is_empty(), "empty string means unbound");
    assert!(!map.contains_key("accent"), "only [keys] is read");
}

#[test]
fn herdr_defaults_used_without_user_config() {
    let defaults = "[keys]\n# prefix = \"ctrl+space\"\n";
    let map = parse_herdr_keys(defaults, None);
    assert_eq!(map["prefix"], vec!["ctrl+space"]);
}

#[test]
fn kitty_user_map_overrides_and_unmaps_defaults() {
    use omarchy_cheatsheets::resolve::parse_kitty_conf;
    let conf = "\
# comment
map shift+insert paste_from_clipboard
map kitty_mod+c no_op
";
    let (cleared, pairs) = parse_kitty_conf(conf);
    assert!(!cleared);
    assert_eq!(
        pairs,
        [
            ("shift+insert".into(), "paste_from_clipboard".into()),
            ("kitty_mod+c".into(), "no_op".into()),
        ]
    );
    let (cleared, _) = parse_kitty_conf("clear_all_shortcuts yes\nmap a show_scrollback\n");
    assert!(cleared);
}

#[test]
fn alacritty_toml_bindings_skip_modal_entries() {
    use omarchy_cheatsheets::resolve::parse_alacritty_conf;
    let conf = r#"
[[keyboard.bindings]]
key = "v"
mods = "Control|Shift"
action = "Paste"

[[keyboard.bindings]]
key = "y"
mode = "Vi"
action = "Copy"

[[keyboard.bindings]]
key = "c"
mods = "Control|Shift"
action = "None"
"#;
    let pairs = parse_alacritty_conf(conf);
    assert_eq!(
        pairs,
        [
            ("ctrl+shift+v".into(), "Paste".into()),
            ("ctrl+shift+c".into(), "None".into()),
        ]
    );
}

#[test]
fn ghostty_keybind_clear_and_action_args() {
    use omarchy_cheatsheets::resolve::parse_ghostty_conf;
    let conf =
        "keybind = clear\nkeybind = ctrl+shift+c=copy_to_clipboard\nkeybind = alt+2=goto_tab:2\n";
    let (cleared, pairs) = parse_ghostty_conf(conf);
    assert!(cleared);
    assert_eq!(
        pairs,
        [
            ("ctrl+shift+c".into(), "copy_to_clipboard".into()),
            ("alt+2".into(), "goto_tab".into()),
        ]
    );
}
