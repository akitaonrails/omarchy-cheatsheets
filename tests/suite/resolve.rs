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
