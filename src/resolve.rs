//! Resolving curated entries against the user's live tool configs, so sheets
//! show the bindings that actually work on this machine — overrides applied,
//! unbound defaults dropped. Tools that can't be probed keep their curated
//! static entries (explicit fallback, per project rules).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::sheet::Sheet;

/// Apply live resolutions per tool.
pub fn apply_resolved_bindings(sheets: &mut [Sheet], config_home: &Path) {
    for sheet in sheets.iter_mut() {
        match sheet.name.as_str() {
            "tmux" => {
                if let Some(keys) = probe_tmux(config_home) {
                    apply_tmux(sheet, &keys);
                }
            }
            "herdr" => {
                if let Some(actions) = resolve_herdr_keys(config_home) {
                    apply_herdr(sheet, &actions);
                }
            }
            "kitty" => {
                if let Some(actions) = resolve_kitty_keys(config_home) {
                    apply_actions(sheet, &actions);
                }
            }
            "alacritty" => {
                if let Some(actions) = resolve_alacritty_keys(config_home) {
                    apply_actions(sheet, &actions);
                }
            }
            "ghostty" => {
                if let Some(actions) = resolve_ghostty_keys(config_home) {
                    apply_actions(sheet, &actions);
                }
            }
            "hyprland" => {
                if let Some(binds) = probe_hyprland() {
                    apply_hyprland(sheet, &binds);
                }
            }
            _ => {}
        }
    }
}

/// tmux/herdr chord notation to display chips: `C-M-Left` / `ctrl+alt+left`
/// both become `["Ctrl", "Alt", "←"]`.
/// tmux/herdr/kitty/ghostty chord notation to display chips: `C-M-Left`,
/// `ctrl+alt+left`, `ctrl+shift+c` → `["Ctrl", "Shift", "c"]`.
///
/// Single letters are modifier names only before the final part — tmux writes
/// `C`/`M`/`S`, but in kitty/ghostty notation the key itself may be `c`, `s`
/// or `m` (`ctrl+shift+c` must not become "Ctrl Shift Ctrl").
pub fn display_chord(chord: &str) -> Vec<String> {
    // tmux separates with '-', herdr/kitty/ghostty with '+'.
    let binding = chord.replace('+', "-");
    let parts: Vec<&str> = binding.split('-').collect();
    let last = parts.len() - 1;
    parts
        .iter()
        .enumerate()
        .map(|(i, part)| {
            let is_modifier_position = i < last;
            match part.to_lowercase().as_str() {
                "c" | "ctrl" | "control" if is_modifier_position => "Ctrl".into(),
                "m" | "alt" | "meta" if is_modifier_position => "Alt".into(),
                "s" | "shift" if is_modifier_position => "Shift".into(),
                "left" | "arrow_left" => "←".into(),
                "right" | "arrow_right" => "→".into(),
                "up" | "arrow_up" => "↑".into(),
                "down" | "arrow_down" => "↓".into(),
                "enter" | "return" => "Enter".into(),
                "space" => "Space".into(),
                "tab" => "Tab".into(),
                "esc" | "escape" => "Esc".into(),
                "backspace" | "bspace" => "Backspace".into(),
                "delete" => "Delete".into(),
                "insert" => "Insert".into(),
                "home" => "Home".into(),
                "end" => "End".into(),
                "pageup" | "page_up" => "PgUp".into(),
                "pagedown" | "page_down" => "PgDn".into(),
                "super" | "cmd" | "command" => "Super".into(),
                _ => part.to_string(),
            }
        })
        .collect()
}

// --- tmux -----------------------------------------------------------------

/// One line of `tmux list-keys`: table, chord, command.
#[derive(Debug, Clone, PartialEq)]
pub struct TmuxKey {
    /// Key table: `prefix`, `root`, `copy-mode-vi`, ...
    pub table: String,
    /// Raw chord, e.g. `C-M-Left`.
    pub chord: String,
    /// Everything after the key, e.g. `split-window -v -c "#{pane_current_path}"`.
    pub command: String,
}

/// Parse `tmux list-keys` output.
pub fn parse_tmux_list_keys(output: &str) -> Vec<TmuxKey> {
    output.lines().filter_map(parse_tmux_key_line).collect()
}

fn parse_tmux_key_line(line: &str) -> Option<TmuxKey> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.first() != Some(&"bind-key") {
        return None;
    }
    let (table, key_idx) = if tokens.get(1) == Some(&"-T") {
        ((*tokens.get(2)?).to_string(), 3)
    } else if tokens.get(1) == Some(&"-n") {
        ("root".to_string(), 2)
    } else {
        return None;
    };
    let chord = (*tokens.get(key_idx)?).to_string();
    let command = tokens.get(key_idx + 1..)?.join(" ");
    Some(TmuxKey {
        table,
        chord,
        command,
    })
}

fn tmux_conf_path(config_home: &Path) -> Option<PathBuf> {
    let xdg = config_home.join("tmux/tmux.conf");
    if xdg.exists() {
        return Some(xdg);
    }
    let home = std::env::var("HOME").ok()?;
    let legacy = PathBuf::from(home).join(".tmux.conf");
    legacy.exists().then_some(legacy)
}

/// Resolve the user's real binding set by booting a throwaway tmux server
/// with their config. A detached probe reflects every override and unbind,
/// unlike the (possibly stale) running server.
fn probe_tmux(config_home: &Path) -> Option<Vec<TmuxKey>> {
    let socket = format!("omarchy-cheatsheets-probe-{}", std::process::id());
    let mut start = Command::new("tmux");
    start.args(["-L", &socket]);
    if let Some(conf) = tmux_conf_path(config_home) {
        start.arg("-f").arg(conf);
    }
    let started = start
        .args(["new-session", "-d", "-s", "probe"])
        .status()
        .ok()?
        .success();
    if !started {
        return None;
    }
    let output = Command::new("tmux")
        .args(["-L", &socket, "list-keys"])
        .output()
        .ok();
    let _ = Command::new("tmux")
        .args(["-L", &socket, "kill-server"])
        .status();
    let output = output.filter(|o| o.status.success())?;
    Some(parse_tmux_list_keys(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn apply_tmux(sheet: &mut Sheet, keys: &[TmuxKey]) {
    for section in &mut sheet.sections {
        section.entries.retain_mut(|entry| {
            let Some(probe) = &entry.probe else {
                return true;
            };
            let Some(command) = probe.command.as_deref() else {
                return true;
            };
            let table = probe.table.as_deref().unwrap_or("prefix");
            let candidates: Vec<&TmuxKey> = keys
                .iter()
                .filter(|k| k.table == table && k.command.contains(command))
                .collect();
            if candidates.is_empty() {
                return false;
            }
            if probe.family {
                return true;
            }
            // Prefer the binding whose chord matches the curated keys — several
            // entries can share a command substring (choose-tree on s and w).
            let curated = if table == "prefix" {
                entry.keys.get(1..).unwrap_or(&[])
            } else {
                &entry.keys
            };
            let found = candidates
                .iter()
                .find(|k| display_chord(&k.chord) == curated)
                .or_else(|| candidates.first())
                .expect("candidates non-empty");
            entry.keys = if table == "prefix" {
                let mut chips = vec!["prefix".to_string()];
                chips.extend(display_chord(&found.chord));
                chips
            } else {
                display_chord(&found.chord)
            };
            true
        });
    }
    sheet.sections.retain(|s| !s.entries.is_empty());
}

// --- herdr ----------------------------------------------------------------

/// Merge herdr's `[keys]` tables: commented defaults from
/// `herdr --default-config` under the user's `config.toml`. An action mapped
/// to an empty value in the user config is unbound, not defaulted.
pub fn parse_herdr_keys(defaults: &str, user_config: Option<&str>) -> HashMap<String, Vec<String>> {
    let mut map = read_herdr_keys_table(defaults, true);
    if let Some(user) = user_config {
        for (action, bindings) in read_herdr_keys_table(user, false) {
            map.insert(action, bindings);
        }
    }
    map
}

fn read_herdr_keys_table(doc: &str, commented: bool) -> HashMap<String, Vec<String>> {
    let mut in_keys = false;
    let mut section = String::from("[keys]\n");
    for line in doc.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if in_keys {
                break;
            }
            in_keys = trimmed == "[keys]";
            continue;
        }
        if !in_keys {
            continue;
        }
        if commented {
            let Some(uncommented) = trimmed.strip_prefix('#') else {
                continue;
            };
            section.push_str(uncommented.trim());
        } else {
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }
            section.push_str(trimmed);
        }
        section.push('\n');
    }
    let Ok(toml::Value::Table(root)) = toml::from_str::<toml::Value>(&section) else {
        return HashMap::new();
    };
    let Some(toml::Value::Table(keys)) = root.get("keys") else {
        return HashMap::new();
    };
    keys.iter()
        .map(|(action, value)| {
            let bindings = match value {
                toml::Value::String(s) if s.is_empty() => Vec::new(),
                toml::Value::String(s) => vec![s.clone()],
                toml::Value::Array(arr) => arr
                    .iter()
                    .filter_map(|b| b.as_str().map(String::from))
                    .collect(),
                _ => Vec::new(),
            };
            (action.clone(), bindings)
        })
        .collect()
}

fn resolve_herdr_keys(config_home: &Path) -> Option<HashMap<String, Vec<String>>> {
    let defaults = Command::new("herdr")
        .arg("--default-config")
        .output()
        .ok()?;
    if !defaults.status.success() {
        return None;
    }
    let user = fs::read_to_string(config_home.join("herdr/config.toml")).ok();
    Some(parse_herdr_keys(
        &String::from_utf8_lossy(&defaults.stdout),
        user.as_deref(),
    ))
}

fn apply_herdr(sheet: &mut Sheet, actions: &HashMap<String, Vec<String>>) {
    apply_actions(sheet, actions);
}

/// Rewrite/drop probed entries from an action → chords map (herdr, kitty,
/// alacritty, ghostty all reduce to this shape).
fn apply_actions(sheet: &mut Sheet, actions: &HashMap<String, Vec<String>>) {
    for section in &mut sheet.sections {
        section.entries.retain_mut(|entry| {
            let Some(probe) = &entry.probe else {
                return true;
            };
            let Some(action) = probe.action.as_deref() else {
                return true;
            };
            let Some(bindings) = actions.get(action) else {
                return false;
            };
            if bindings.is_empty() {
                return false;
            }
            if probe.family {
                return true;
            }
            let chosen = match &probe.prefer {
                Some(prefer) => bindings
                    .iter()
                    .find(|b| b.contains(prefer.as_str()))
                    .unwrap_or(&bindings[0]),
                None => &bindings[0],
            };
            entry.keys = display_chord(chosen);
            true
        });
    }
    sheet.sections.retain(|s| !s.entries.is_empty());
}

// --- prefix / leader resolution ------------------------------------------

/// Substitute `{prefix}` in sheet titles with the key resolved from the
/// user's actual config files. Sheets whose tool has no discoverable
/// prefix fall back to their static title (template stripped).
pub fn apply_dynamic_prefixes(sheets: &mut [Sheet], config_home: &Path) {
    for sheet in sheets.iter_mut() {
        let resolved = resolve_prefix(&sheet.name, config_home);
        let title = sheet.title.take().unwrap_or_else(|| sheet.name.clone());
        sheet.title = Some(render_title(&title, resolved.as_deref()));
    }
}

fn render_title(template: &str, prefix: Option<&str>) -> String {
    match prefix {
        Some(p) => template.replace("{prefix}", p),
        None => template
            .replace(" (prefix: {prefix})", "")
            .replace(", leader: {prefix})", ")")
            .replace("{prefix}", "prefix"),
    }
}

/// Resolve a tool's prefix/leader from the user's config. Returns a display
/// string like `Ctrl+Space / Ctrl+b`.
fn resolve_prefix(sheet: &str, config_home: &Path) -> Option<String> {
    match sheet {
        "tmux" => resolve_tmux_prefix(config_home),
        "herdr" => resolve_herdr_prefix(config_home),
        "nvim" => resolve_nvim_leader(config_home),
        _ => None,
    }
}

/// tmux chord notation (`C-Space`, `M-Enter`) to display form.
fn tmux_chord(chord: &str) -> String {
    display_chord(chord).join("+")
}

fn resolve_tmux_prefix(config_home: &Path) -> Option<String> {
    let conf = config_home.join("tmux/tmux.conf");
    let conf = if conf.exists() {
        conf
    } else {
        // Pre-XDG location
        let home = std::env::var("HOME").ok()?;
        PathBuf::from(home).join(".tmux.conf")
    };
    let source = fs::read_to_string(conf).ok()?;

    let mut prefixes = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("set -g prefix ") {
            prefixes.push(tmux_chord(rest.trim()));
        } else if let Some(rest) = line.strip_prefix("set -g prefix2 ") {
            prefixes.push(tmux_chord(rest.trim()));
        }
    }
    (!prefixes.is_empty()).then(|| prefixes.join(" / "))
}

fn resolve_herdr_prefix(config_home: &Path) -> Option<String> {
    let conf = config_home.join("herdr/config.toml");
    let source = fs::read_to_string(conf).ok()?;
    let doc: toml::Value = toml::from_str(&source).ok()?;
    let prefix = doc.get("keys")?.get("prefix")?.as_str()?;
    Some(display_chord(prefix).join("+"))
}

/// LazyVim reads `vim.g.mapleader`; default is Space. Scan the user's lua
/// config for an override.
fn resolve_nvim_leader(config_home: &Path) -> Option<String> {
    let lua_dir = config_home.join("nvim/lua");
    let source = read_lua_sources(&lua_dir)?;
    for line in source.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("vim.g.mapleader") {
            let value = rest
                .trim_start_matches(['=', ' ', '"'])
                .trim_end_matches(['"', ' ']);
            return Some(match value {
                " " | "" => "Space".to_string(),
                other => other.to_string(),
            });
        }
    }
    Some("Space".to_string())
}

fn read_lua_sources(dir: &Path) -> Option<String> {
    let mut out = String::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "lua") {
                out.push_str(&fs::read_to_string(path).unwrap_or_default());
                out.push('\n');
            }
        }
    }
    (!out.is_empty()).then_some(out)
}

// --- terminal emulators (kitty / alacritty / ghostty) ----------------------
//
// All three keep defaults outside the config file, so resolution is:
// embedded defaults for the curated actions, overlaid with the user's config
// (later lines win per key; explicit no-op/unmap/clear drops bindings).

/// Default chord for each curated action, in kitty's config notation.
const KITTY_DEFAULTS: &[(&str, &str)] = &[
    ("kitty_mod+c", "copy_to_clipboard"),
    ("kitty_mod+v", "paste_from_clipboard"),
    ("kitty_mod+s", "paste_from_selection"),
    ("shift+insert", "paste_from_selection"),
    ("kitty_mod+up", "scroll_line_up"),
    ("kitty_mod+down", "scroll_line_down"),
    ("kitty_mod+page_up", "scroll_page_up"),
    ("kitty_mod+page_down", "scroll_page_down"),
    ("kitty_mod+home", "scroll_home"),
    ("kitty_mod+end", "scroll_end"),
    ("kitty_mod+h", "show_scrollback"),
    ("kitty_mod+g", "show_last_command_output"),
    ("kitty_mod+enter", "new_window"),
    ("kitty_mod+w", "close_window"),
    ("kitty_mod+]", "next_window"),
    ("kitty_mod+[", "previous_window"),
    ("kitty_mod+f", "move_window"),
    ("kitty_mod+b", "move_window"),
    ("kitty_mod+n", "new_os_window"),
    ("kitty_mod+l", "next_layout"),
    ("kitty_mod+t", "new_tab"),
    ("kitty_mod+q", "close_tab"),
    ("kitty_mod+right", "next_tab"),
    ("kitty_mod+left", "previous_tab"),
    ("ctrl+tab", "next_tab"),
    ("ctrl+shift+tab", "previous_tab"),
    ("kitty_mod+period", "move_tab_forward"),
    ("kitty_mod+comma", "move_tab_backward"),
    ("kitty_mod+equal", "increase_font_size"),
    ("kitty_mod+minus", "decrease_font_size"),
    ("kitty_mod+backspace", "reset_font_size"),
    ("kitty_mod+f11", "toggle_fullscreen"),
    ("kitty_mod+f10", "toggle_maximized"),
    ("kitty_mod+f2", "edit_config_file"),
    ("kitty_mod+f5", "reload_config_file"),
];

/// Alacritty defaults (Linux), converted from `mods: "Control|Shift"` to our
/// chord notation. Action names match bindings.rs.
const ALACRITTY_DEFAULTS: &[(&str, &str)] = &[
    ("ctrl+shift+c", "Copy"),
    ("ctrl+shift+v", "Paste"),
    ("shift+insert", "PasteSelection"),
    ("shift+pageup", "ScrollPageUp"),
    ("shift+pagedown", "ScrollPageDown"),
    ("shift+home", "ScrollToTop"),
    ("shift+end", "ScrollToBottom"),
    ("ctrl+shift+f", "SearchForward"),
    ("ctrl+shift+b", "SearchBackward"),
    ("ctrl+shift+space", "ToggleViMode"),
    ("ctrl+equal", "IncreaseFontSize"),
    ("ctrl+minus", "DecreaseFontSize"),
    ("ctrl+0", "ResetFontSize"),
];

/// Ghostty defaults (Linux), from Keybinds.init() in src/config/Config.zig.
const GHOSTTY_DEFAULTS: &[(&str, &str)] = &[
    ("ctrl+shift+c", "copy_to_clipboard"),
    ("ctrl+insert", "copy_to_clipboard"),
    ("ctrl+shift+v", "paste_from_clipboard"),
    ("shift+insert", "paste_from_selection"),
    ("shift+home", "scroll_to_top"),
    ("shift+end", "scroll_to_bottom"),
    ("shift+pageup", "scroll_page_up"),
    ("shift+pagedown", "scroll_page_down"),
    ("ctrl+shift+arrow_up", "jump_to_prompt"),
    ("ctrl+shift+arrow_down", "jump_to_prompt"),
    ("ctrl+shift+f", "start_search"),
    ("escape", "end_search"),
    ("ctrl+shift+t", "new_tab"),
    ("ctrl+shift+w", "close_surface"),
    ("ctrl+tab", "next_tab"),
    ("ctrl+shift+tab", "previous_tab"),
    ("ctrl+shift+arrow_right", "next_tab"),
    ("ctrl+shift+arrow_left", "previous_tab"),
    ("ctrl+shift+page_up", "move_tab"),
    ("ctrl+shift+page_down", "move_tab"),
    ("alt+1", "goto_tab"),
    ("alt+2", "goto_tab"),
    ("alt+3", "goto_tab"),
    ("alt+4", "goto_tab"),
    ("alt+5", "goto_tab"),
    ("alt+6", "goto_tab"),
    ("alt+7", "goto_tab"),
    ("alt+8", "goto_tab"),
    ("alt+9", "last_tab"),
    ("ctrl+shift+n", "new_window"),
    ("alt+f4", "close_window"),
    ("ctrl+shift+q", "quit"),
    ("ctrl+shift+o", "new_split"),
    ("ctrl+shift+e", "new_split"),
    ("ctrl+super+[", "goto_split"),
    ("ctrl+super+]", "goto_split"),
    ("ctrl+alt+arrow_up", "goto_split"),
    ("ctrl+alt+arrow_down", "goto_split"),
    ("ctrl+alt+arrow_left", "goto_split"),
    ("ctrl+alt+arrow_right", "goto_split"),
    ("ctrl+super+shift+arrow_up", "resize_split"),
    ("ctrl+super+shift+arrow_down", "resize_split"),
    ("ctrl+super+shift+arrow_left", "resize_split"),
    ("ctrl+super+shift+arrow_right", "resize_split"),
    ("ctrl+shift+enter", "toggle_split_zoom"),
    ("ctrl+equal", "increase_font_size"),
    ("ctrl+minus", "decrease_font_size"),
    ("ctrl+0", "reset_font_size"),
    ("ctrl+enter", "toggle_fullscreen"),
    ("ctrl+shift+p", "toggle_command_palette"),
    ("ctrl+shift+comma", "reload_config"),
    ("ctrl+comma", "open_config"),
    ("ctrl+shift+a", "select_all"),
    ("ctrl+shift+alt+j", "write_screen_file"),
    ("ctrl+shift+j", "write_screen_file"),
];

/// Parse kitty's config: `map <chord> <action> [args]` and `unmap <chord>`.
/// Returns (cleared, ordered (chord, action) pairs).
pub fn parse_kitty_conf(source: &str) -> (bool, Vec<(String, String)>) {
    let mut cleared = false;
    let mut out = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("clear_all_shortcuts") {
            cleared = line.ends_with("yes") || line.ends_with("true");
            continue;
        }
        if let Some(rest) = line.strip_prefix("unmap ") {
            if let Some(chord) = rest.split_whitespace().next() {
                out.push((chord.to_string(), "no_op".to_string()));
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("map ") {
            let mut tokens = rest.split_whitespace();
            let (Some(chord), Some(action)) = (tokens.next(), tokens.next()) else {
                continue;
            };
            out.push((chord.to_string(), action.to_string()));
        }
    }
    (cleared, out)
}

/// kitty's `kitty_mod` setting, defaulting to ctrl+shift.
fn kitty_mod(source: &str) -> String {
    let mut found = "ctrl+shift".to_string();
    for line in source.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("kitty_mod ")
            && let Some(m) = rest.split_whitespace().next()
        {
            found = m.to_string();
        }
    }
    found
}

/// Parse alacritty's TOML `[[keyboard.bindings]]`; only normal-mode entries.
/// Returns ordered (chord, action) pairs; action `None` unbinds.
pub fn parse_alacritty_conf(source: &str) -> Vec<(String, String)> {
    let Ok(doc) = toml::from_str::<toml::Value>(source) else {
        return Vec::new();
    };
    let Some(bindings) = doc
        .get("keyboard")
        .and_then(|k| k.get("bindings"))
        .and_then(toml::Value::as_array)
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for binding in bindings {
        // Modal bindings (Vi/Search) are excluded — the sheet covers normal mode.
        if binding.get("mode").is_some() {
            continue;
        }
        let (Some(key), Some(action)) = (
            binding.get("key").and_then(toml::Value::as_str),
            binding.get("action").and_then(toml::Value::as_str),
        ) else {
            continue;
        };
        let mods = binding
            .get("mods")
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        let mut parts: Vec<String> = mods
            .split('|')
            .filter(|m| !m.is_empty())
            .map(|m| {
                // alacritty writes "Control"; our notation uses "ctrl"
                match m.to_lowercase().as_str() {
                    "control" => "ctrl".to_string(),
                    other => other.to_string(),
                }
            })
            .collect();
        let key = if key.len() == 1 {
            key.to_lowercase()
        } else {
            key.to_string()
        };
        parts.push(key);
        out.push((parts.join("+"), action.to_string()));
    }
    out
}

/// Parse ghostty's config: `keybind = trigger=action[:args]` and
/// `keybind = clear`. Returns (cleared, ordered (chord, action) pairs);
/// action args (`goto_tab:2`) are trimmed to the action name.
pub fn parse_ghostty_conf(source: &str) -> (bool, Vec<(String, String)>) {
    let mut cleared = false;
    let mut out = Vec::new();
    for line in source.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some(rest) = line.strip_prefix("keybind") else {
            continue;
        };
        let Some(rest) = rest.trim().strip_prefix('=').map(str::trim) else {
            continue;
        };
        if rest == "clear" {
            cleared = true;
            continue;
        }
        let Some((trigger, action)) = rest.split_once('=') else {
            continue;
        };
        let action = action.split(':').next().unwrap_or(action).trim();
        out.push((trigger.trim().to_string(), action.to_string()));
    }
    (cleared, out)
}

/// Overlay ordered (chord, action) pairs on the defaults: the last owner of a
/// chord wins; `no_op`/`None` unbind. Returns action → chords, defaults first
/// (deterministic order so the curated default chord leads each entry).
fn overlay_keymap(
    defaults: &[(&str, &str)],
    cleared: bool,
    user: &[(String, String)],
) -> HashMap<String, Vec<String>> {
    let mut by_key: HashMap<String, &str> = HashMap::new();
    if !cleared {
        for (chord, action) in defaults {
            by_key.insert(chord.to_string(), action);
        }
    }
    for (chord, action) in user {
        by_key.insert(chord.clone(), action);
    }

    let mut actions: HashMap<String, Vec<String>> = HashMap::new();
    let mut collect = |chord: &str, action: &str, by_key: &HashMap<String, &str>| {
        if matches!(action, "no_op" | "None") {
            return;
        }
        // A chord lands under its final owner only.
        if by_key.get(chord) == Some(&action) {
            actions
                .entry(action.to_string())
                .or_default()
                .push(chord.to_string());
        }
    };
    if !cleared {
        for (chord, action) in defaults {
            collect(chord, action, &by_key);
        }
    }
    for (chord, action) in user {
        collect(chord, action, &by_key);
    }
    actions
}

fn resolve_kitty_keys(config_home: &Path) -> Option<HashMap<String, Vec<String>>> {
    let source = fs::read_to_string(config_home.join("kitty/kitty.conf")).unwrap_or_default();
    let modifier = kitty_mod(&source);
    let (cleared, user) = parse_kitty_conf(&source);
    let user: Vec<(String, String)> = user
        .into_iter()
        .map(|(chord, action)| (chord.replace("kitty_mod", &modifier), action))
        .collect();
    let defaults: Vec<(String, String)> = KITTY_DEFAULTS
        .iter()
        .map(|(c, a)| (c.replace("kitty_mod", &modifier), a.to_string()))
        .collect();
    Some(overlay_keymap(
        &defaults
            .iter()
            .map(|(c, a)| (c.as_str(), a.as_str()))
            .collect::<Vec<_>>(),
        cleared,
        &user,
    ))
}

fn resolve_alacritty_keys(config_home: &Path) -> Option<HashMap<String, Vec<String>>> {
    let source =
        fs::read_to_string(config_home.join("alacritty/alacritty.toml")).unwrap_or_default();
    let user = parse_alacritty_conf(&source);
    Some(overlay_keymap(ALACRITTY_DEFAULTS, false, &user))
}

fn resolve_ghostty_keys(config_home: &Path) -> Option<HashMap<String, Vec<String>>> {
    let source = fs::read_to_string(config_home.join("ghostty/config")).unwrap_or_default();
    let (cleared, user) = parse_ghostty_conf(&source);
    Some(overlay_keymap(GHOSTTY_DEFAULTS, cleared, &user))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sheet::{Entry, Probe, Section};

    fn entry(keys: &[&str], probe: Option<Probe>) -> Entry {
        Entry {
            keys: keys.iter().map(|s| (*s).to_string()).collect(),
            description: "test".into(),
            tags: vec![],
            probe,
        }
    }

    fn probe(command: &str, table: &str) -> Option<Probe> {
        Some(Probe {
            command: Some(command.to_string()),
            table: Some(table.to_string()),
            ..Default::default()
        })
    }

    fn sheet_with(entries: Vec<Entry>) -> Sheet {
        Sheet {
            name: "tmux".into(),
            title: None,
            icon: None,
            command: None,
            sections: vec![Section {
                name: "S".into(),
                entries,
            }],
        }
    }

    #[test]
    fn tmux_rewrites_keys_to_the_real_chord() {
        // User remapped split-window -v to prefix b.
        let keys = parse_tmux_list_keys("bind-key -T prefix b split-window -v\n");
        let mut sheet = sheet_with(vec![entry(
            &["prefix", "h"],
            probe("split-window -v", "prefix"),
        )]);
        apply_tmux(&mut sheet, &keys);
        assert_eq!(sheet.sections[0].entries[0].keys, ["prefix", "b"]);
    }

    #[test]
    fn tmux_drops_defaults_the_user_unbound() {
        let keys = parse_tmux_list_keys("bind-key -T prefix x kill-pane\n");
        let mut sheet = sheet_with(vec![
            entry(&["prefix", "x"], probe("kill-pane", "prefix")),
            entry(&["prefix", "z"], probe("resize-pane -Z", "prefix")),
            entry(&["static"], None),
        ]);
        apply_tmux(&mut sheet, &keys);
        let kept: Vec<_> = sheet.sections[0]
            .entries
            .iter()
            .map(|e| e.keys[0].clone())
            .collect();
        assert_eq!(
            kept,
            ["prefix", "static"],
            "unbound zoom dropped, unprobed kept"
        );
    }

    #[test]
    fn tmux_family_keeps_curated_keys_when_anything_matches() {
        let keys = parse_tmux_list_keys("bind-key -T root C-M-Left select-pane -L\n");
        let mut sheet = sheet_with(vec![entry(
            &["Ctrl", "Alt", "←/→/↑/↓"],
            Some(Probe {
                command: Some("select-pane".into()),
                table: Some("root".into()),
                family: true,
                ..Default::default()
            }),
        )]);
        apply_tmux(&mut sheet, &keys);
        assert_eq!(
            sheet.sections[0].entries[0].keys,
            ["Ctrl", "Alt", "←/→/↑/↓"]
        );
    }

    #[test]
    fn tmux_prefers_the_curated_chord_among_shared_commands() {
        let keys = parse_tmux_list_keys(
            "bind-key -T prefix s choose-tree -s\nbind-key -T prefix w choose-tree -w\n",
        );
        let mut sheet = sheet_with(vec![entry(
            &["prefix", "w"],
            probe("choose-tree", "prefix"),
        )]);
        apply_tmux(&mut sheet, &keys);
        assert_eq!(sheet.sections[0].entries[0].keys, ["prefix", "w"]);
    }

    #[test]
    fn herdr_rewrites_with_prefer_and_drops_unbound() {
        let mut actions = HashMap::new();
        actions.insert(
            "split_horizontal".to_string(),
            vec!["prefix+b".to_string(), "alt+enter".to_string()],
        );
        actions.insert("zoom".to_string(), Vec::new());

        let mut sheet = sheet_with(vec![
            entry(
                &["Alt", "Enter"],
                Some(Probe {
                    action: Some("split_horizontal".into()),
                    prefer: Some("alt".into()),
                    ..Default::default()
                }),
            ),
            entry(
                &["prefix", "z"],
                Some(Probe {
                    action: Some("zoom".into()),
                    ..Default::default()
                }),
            ),
        ]);
        apply_herdr(&mut sheet, &actions);
        assert_eq!(sheet.sections[0].entries.len(), 1);
        assert_eq!(sheet.sections[0].entries[0].keys, ["Alt", "Enter"]);
    }

    #[test]
    fn overlay_default_chord_leads_and_stolen_keys_move() {
        let defaults = [
            ("kitty_mod+v", "paste_from_clipboard"),
            ("shift+insert", "paste_from_selection"),
        ];
        let user = vec![(
            "shift+insert".to_string(),
            "paste_from_clipboard".to_string(),
        )];
        let map = overlay_keymap(&defaults, false, &user);
        assert_eq!(
            map["paste_from_clipboard"],
            ["kitty_mod+v", "shift+insert"],
            "default chord leads, user's chord follows"
        );
        assert!(
            !map.contains_key("paste_from_selection"),
            "stolen action loses its only chord and drops out"
        );
    }

    #[test]
    fn overlay_clear_all_drops_defaults() {
        let defaults = [("kitty_mod+c", "copy_to_clipboard")];
        let user = vec![("ctrl+alt+c".to_string(), "copy_to_clipboard".to_string())];
        let map = overlay_keymap(&defaults, true, &user);
        assert_eq!(map["copy_to_clipboard"], ["ctrl+alt+c"]);
    }

    #[test]
    fn overlay_no_op_unbinds() {
        let defaults = [("kitty_mod+c", "copy_to_clipboard")];
        let user = vec![("kitty_mod+c".to_string(), "no_op".to_string())];
        let map = overlay_keymap(&defaults, false, &user);
        assert!(!map.contains_key("copy_to_clipboard"));
    }
}

// --- hyprland ---------------------------------------------------------------

/// One live Hyprland binding from `hyprctl binds -j`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct HyprBind {
    /// Modifier bitmask (Shift=1, Ctrl=4, Alt=8, Super=64).
    pub modmask: u64,
    /// Key name as Hyprland reports it: `K`, `RETURN`, `SLASH`, `code:20`.
    pub key: String,
    /// Quattro binds carry human descriptions; empty for raw `bind` lines.
    #[serde(default)]
    pub description: String,
}

/// Parse `hyprctl binds -j` output, skipping mouse bindings.
pub fn parse_hyprctl_binds(json: &str) -> Vec<HyprBind> {
    serde_json::from_str::<Vec<HyprBind>>(json)
        .unwrap_or_default()
        .into_iter()
        .filter(|b| !b.key.starts_with("mouse"))
        .collect()
}

/// Live Hyprland bindings from the running compositor. The compositor already
/// merged stock defaults and user Lua, so this is the true effective set.
fn probe_hyprland() -> Option<Vec<HyprBind>> {
    let output = Command::new("hyprctl")
        .args(["binds", "-j"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| parse_hyprctl_binds(&String::from_utf8_lossy(&output.stdout)))
}

/// Modifier bitmask to chips, in the order Omarchy writes them.
fn modmask_chips(modmask: u64) -> Vec<String> {
    let mut out = Vec::new();
    if modmask & 64 != 0 {
        out.push("Super".to_string());
    }
    if modmask & 4 != 0 {
        out.push("Ctrl".to_string());
    }
    if modmask & 8 != 0 {
        out.push("Alt".to_string());
    }
    if modmask & 1 != 0 {
        out.push("Shift".to_string());
    }
    out
}

/// Hyprland key names to chip text.
fn hypr_key_chip(key: &str) -> String {
    match key {
        "RETURN" => "Enter".into(),
        "SPACE" => "Space".into(),
        "ESCAPE" => "Esc".into(),
        "TAB" => "Tab".into(),
        "BACKSPACE" => "Backspace".into(),
        "PRINT" => "Print".into(),
        "SLASH" => "/".into(),
        "BACKSLASH" => "\\".into(),
        "COMMA" => ",".into(),
        "PERIOD" => ".".into(),
        "MINUS" => "-".into(),
        "EQUAL" => "=".into(),
        "BRACKETLEFT" => "[".into(),
        "BRACKETRIGHT" => "]".into(),
        "SEMICOLON" => ";".into(),
        "APOSTROPHE" => "'".into(),
        "LEFT" => "←".into(),
        "RIGHT" => "→".into(),
        "UP" => "↑".into(),
        "DOWN" => "↓".into(),
        // evdev codes Quattro uses for punctuation on some layouts
        "code:20" => "-".into(),
        "code:21" => "=".into(),
        "code:34" => "[".into(),
        "code:35" => "]".into(),
        "XF86AudioRaiseVolume" => "Vol+".into(),
        "XF86AudioLowerVolume" => "Vol-".into(),
        "XF86AudioMute" => "Mute".into(),
        "XF86AudioMicMute" => "Mic mute".into(),
        "XF86AudioPlay" => "Play".into(),
        "XF86AudioPause" => "Pause".into(),
        "XF86AudioNext" => "Next".into(),
        "XF86AudioPrev" => "Prev".into(),
        "XF86MonBrightnessUp" => "Bright+".into(),
        "XF86MonBrightnessDown" => "Bright-".into(),
        other => {
            // Single letters come uppercase already; leave everything else as-is.
            other.to_string()
        }
    }
}

/// Hyprland entries probe by description substring (Quattro binds carry them).
fn apply_hyprland(sheet: &mut Sheet, binds: &[HyprBind]) {
    for section in &mut sheet.sections {
        section.entries.retain_mut(|entry| {
            let Some(probe) = &entry.probe else {
                return true;
            };
            let Some(want) = probe.command.as_deref() else {
                return true;
            };
            let found = binds
                .iter()
                .find(|b| b.description.to_lowercase().contains(&want.to_lowercase()));
            let Some(found) = found else { return false };
            if probe.family {
                return true;
            }
            let mut chips = modmask_chips(found.modmask);
            chips.push(hypr_key_chip(&found.key));
            entry.keys = chips;
            true
        });
    }
    sheet.sections.retain(|s| !s.entries.is_empty());
}
