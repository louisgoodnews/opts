//! Tests of the scan status texts: every level and reason has its message in both languages and an
//! icon, and the chip colours read in every theme.

use super::*;

#[test]
fn every_level_and_reason_has_text_in_both_languages_and_an_icon() {
    let mut loc = Localization::new("en");
    loc.extend(
        "en",
        include_str!("../../../settings_registry/messages_en.ftl"),
    );
    loc.extend(
        "de",
        include_str!("../../../settings_registry/messages_de.ftl"),
    );
    let reasons = [
        ScanReason::FolderMissing,
        ScanReason::Failed,
        ScanReason::NeverScanned,
        ScanReason::Empty,
        ScanReason::Running,
        ScanReason::FolderChanged,
        ScanReason::Older,
        ScanReason::Recent,
    ];
    for lang in ["en", "de", "en-XA"] {
        loc.set_locale(lang);
        for r in reasons {
            assert!(!loc.t(reason_key(r)).contains('⟦'), "{lang} {r:?}");
        }
        for l in [ScanLevel::Good, ScanLevel::Recommended, ScanLevel::Needed] {
            assert!(!loc.t(level_key(l)).contains('⟦'), "{lang} {l:?}");
            assert!(!look(l).2.is_empty());
        }
        assert!(!loc.t("scan-never").contains('⟦'));
    }
    // The three levels differ by word and by icon, not only by colour.
    assert_ne!(look(ScanLevel::Good).2, look(ScanLevel::Needed).2);
    assert_ne!(look(ScanLevel::Good).2, look(ScanLevel::Recommended).2);
    assert_ne!(level_key(ScanLevel::Good), level_key(ScanLevel::Needed));
}

#[test]
fn the_chip_colours_are_readable_in_every_theme() {
    use opts_gui::theme::contrast::contrast_ratio;
    use opts_gui::theme::{ThemeKind, ThemePreferences};
    let prefs = ThemePreferences::default();
    for kind in [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast] {
        let scheme = prefs.scheme_of(kind);
        let need = if kind == ThemeKind::HighContrast {
            7.0
        } else {
            4.5
        };
        for l in [ScanLevel::Good, ScanLevel::Recommended, ScanLevel::Needed] {
            let (bg, fg, _) = look(l);
            let ratio = contrast_ratio(scheme.get(fg), scheme.get(bg));
            assert!(ratio >= need, "{kind:?} {l:?}: {ratio:.2} < {need}");
        }
    }
}
