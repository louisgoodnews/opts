//! Unit tests of `scan_status.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

const DAY: i64 = 86_400;

fn base() -> ScanInputs {
    ScanInputs {
        now: 100 * DAY,
        last_scan: Some(100 * DAY - 3600),
        entities: 75_313,
        interval_days: 1,
        root: RootState::Present,
        folder_changed: false,
        failed: false,
        running: false,
    }
}

fn verdict(i: ScanInputs) -> (ScanLevel, ScanReason) {
    let s = assess(&i);
    (s.level, s.reason)
}

#[test]
fn the_rules_in_order() {
    use ScanLevel::*;
    use ScanReason as R;
    assert_eq!(verdict(base()), (Good, R::Recent));
    let b = base();
    assert_eq!(
        verdict(ScanInputs {
            root: RootState::Unset,
            ..b
        }),
        (Needed, R::FolderMissing)
    );
    assert_eq!(
        verdict(ScanInputs {
            root: RootState::Missing,
            ..b
        }),
        (Needed, R::FolderMissing)
    );
    assert_eq!(
        verdict(ScanInputs { failed: true, ..b }),
        (Needed, R::Failed)
    );
    assert_eq!(
        verdict(ScanInputs {
            last_scan: None,
            ..b
        }),
        (Needed, R::NeverScanned)
    );
    assert_eq!(
        verdict(ScanInputs {
            last_scan: None,
            running: true,
            ..b
        }),
        (Needed, R::Running)
    );
    assert_eq!(verdict(ScanInputs { entities: 0, ..b }), (Needed, R::Empty));
    assert_eq!(
        verdict(ScanInputs { running: true, ..b }),
        (Recommended, R::Running)
    );
    assert_eq!(
        verdict(ScanInputs {
            folder_changed: true,
            ..b
        }),
        (Recommended, R::FolderChanged)
    );
    assert_eq!(
        verdict(ScanInputs {
            last_scan: Some(b.now - 2 * DAY),
            ..b
        }),
        (Recommended, R::Older)
    );
    assert_eq!(
        verdict(ScanInputs {
            last_scan: Some(b.now - 2 * DAY),
            interval_days: 7,
            ..b
        }),
        (Good, R::Recent),
        "within a 7 day interval"
    );
    // Red beats yellow.
    assert_eq!(
        verdict(ScanInputs {
            folder_changed: true,
            failed: true,
            ..b
        }),
        (Needed, R::Failed)
    );
}

#[test]
fn a_changed_top_level_folder_is_detected_cheaply() {
    let root = std::env::temp_dir().join(format!("opts-scanstatus-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("Vehicles")).unwrap();
    let mtime = |p: &Path, secs: u64| {
        let f = std::fs::File::open(p).unwrap();
        f.set_modified(UNIX_EPOCH + std::time::Duration::from_secs(secs))
            .unwrap();
    };
    mtime(&root.join("Vehicles"), 1_000);
    mtime(&root, 1_000);
    assert!(!folder_changed_since(&root, 2_000));
    mtime(&root.join("Vehicles"), 3_000);
    assert!(
        folder_changed_since(&root, 2_000),
        "a new vehicle folder touches Vehicles"
    );
    mtime(&root.join("Vehicles"), 1_000);
    mtime(&root, 3_000);
    assert!(folder_changed_since(&root, 2_000));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn every_level_and_reason_has_text_in_both_languages_and_an_icon() {
    let mut loc = Localization::new("en");
    loc.extend(
        "en",
        include_str!("../../settings_registry/messages_en.ftl"),
    );
    loc.extend(
        "de",
        include_str!("../../settings_registry/messages_de.ftl"),
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
