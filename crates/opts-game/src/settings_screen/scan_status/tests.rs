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
mod folder_tests;
mod text_tests;
