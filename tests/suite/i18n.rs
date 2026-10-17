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
