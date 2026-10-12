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
            "enter" | "return" => "Enter".into(),
            "space" => "Space".into(),
            "tab" => "Tab".into(),
            "esc" | "escape" => "Esc".into(),
            "backspace" | "bspace" => "Backspace".into(),
            "delete" => "Delete".into(),
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
}
