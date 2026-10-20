//! UI translations: known text translates per locale, anything else stays English.

use omarchy_cheatsheets::i18n;

#[test]
fn portuguese_locales_translate() {
    assert_eq!(i18n::translate("pt_BR", "Appearance"), "Aparência");
    assert_eq!(i18n::translate("pt", "to close"), "para fechar");
}

#[test]
fn unknown_text_or_locale_stays_english() {
    assert_eq!(
        i18n::translate("pt_BR", "Not in the table"),
        "Not in the table"
    );
    assert_eq!(i18n::translate("de_DE", "Appearance"), "Appearance");
    assert_eq!(i18n::translate("", "Appearance"), "Appearance");
}

#[test]
fn every_qml_tr_string_has_a_translation() {
    // A literal missed by the table silently stays English, so the QML ↔
    // table mirror needs a contract test like qml_keys_cover_every_color.
    for file in ["Main.qml", "QuickSearch.qml"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("qml")
            .join(file);
        let source = std::fs::read_to_string(path).unwrap();
        let literals = tr_literals(&source);
        assert!(!literals.is_empty(), "{file}: no bridge.tr() calls found");
        for literal in literals {
            assert!(
                i18n::has_translation("pt_BR", &literal),
                "{file}: no pt-BR entry for {literal:?}"
            );
        }
    }
}

/// String literals passed to `bridge.tr(...)` in `source`, `\"` unescaped.
fn tr_literals(source: &str) -> Vec<String> {
    const MARKER: &str = "bridge.tr(\"";
    let mut out = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find(MARKER) {
        let tail = &rest[start + MARKER.len()..];
        let mut literal = String::new();
        let mut escaped = false;
        for c in tail.chars() {
            match (escaped, c) {
                (true, _) => {
                    literal.push(c); // drop the backslash: \" reads as "
                    escaped = false;
                }
                (false, '\\') => escaped = true,
                (false, '"') => break,
                (false, _) => literal.push(c),
            }
        }
        out.push(literal);
        rest = tail;
    }
    out
}
