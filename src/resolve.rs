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
            _ => {}
        }
    }
}

/// tmux/herdr chord notation to display chips: `C-M-Left` / `ctrl+alt+left`
/// both become `["Ctrl", "Alt", "←"]`.
pub fn display_chord(chord: &str) -> Vec<String> {
    // tmux separates with '-', herdr with '+'.
    chord
        .replace('+', "-")
        .split('-')
        .map(|part| match part.to_lowercase().as_str() {
            "c" | "ctrl" | "control" => "Ctrl".into(),
            "m" | "alt" | "meta" => "Alt".into(),
            "s" | "shift" => "Shift".into(),
            "left" => "←".into(),
            "right" => "→".into(),
            "up" => "↑".into(),
            "down" => "↓".into(),
            _ => part.to_string(),
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
