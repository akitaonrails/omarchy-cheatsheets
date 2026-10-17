//! UI translations: QML passes its English text through `bridge.tr()`, which
//! looks it up for the user's locale. Missing text stays English, so a partial
//! table never blanks the window.

/// Brazilian Portuguese, keyed by the English text in the QML.
const PT_BR: &[(&str, &str)] = &[
    ("All sheets", "Todos"),
    ("Cheatsheets", "Atalhos"),
    ("Type to filter bindings…", "Digite para filtrar…"),
    (
        "No bindings match — try a shorter query",
        "Nada encontrado — tente uma busca menor",
    ),
    (
        "Search bindings across all apps…",
        "Buscar atalhos em todos os programas…",
    ),
    (
        "No bindings match — try fewer words",
        "Nada encontrado — tente menos palavras",
    ),
    (
        "Type what it does, its section or app — e.g. \"split pane\", \"copy mode\", \"tmux quit\"",
        "Digite o que o atalho faz, a seção ou o programa — ex.: \"dividir\", \"copiar\", \"kitty aba\"",
    ),
    ("type", "digite"),
    ("to filter", "para filtrar"),
    ("to search", "para buscar"),
    ("to move", "para mover"),
    ("page", "página"),
    ("next app", "próximo programa"),
    ("scroll", "rolar"),
    ("click", "clique"),
    ("to scope", "para escolher"),
    ("settings", "aparência"),
    ("to close", "para fechar"),
    ("Appearance", "Aparência"),
    ("Grep in project", "Buscar no projeto"),
    ("Text", "Texto"),
    ("System default", "Padrão do sistema"),
    ("Size", "Tamanho"),
    ("Window", "Janela"),
    ("Opacity", "Opacidade"),
    ("Theme", "Tema"),
    ("Saved to ", "Salvo em "),
    ("Reset", "Restaurar"),
];

/// Language of the UI from the usual locale variables (`pt_BR.UTF-8` → `pt_BR`).
pub fn locale() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|var| std::env::var(var).ok())
        .find(|v| !v.is_empty())
        .map(|v| v.split(['.', '@']).next().unwrap_or_default().to_string())
        .unwrap_or_default()
}

/// `text` translated for `locale`, or `text` itself when there is no translation.
pub fn translate(locale: &str, text: &str) -> String {
    let table = match locale.split('_').next() {
        Some("pt") => PT_BR,
        _ => &[],
    };
    table
        .iter()
        .find(|(english, _)| *english == text)
        .map_or(text, |(_, translated)| translated)
        .to_string()
}
