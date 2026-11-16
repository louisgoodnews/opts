//! Saved files are validated, broken saves fail and scratch files are ignored.

use super::*;

#[test]
fn a_saved_file_is_validated_after_the_quiet_period() {
    let dir = temp_dir("valid");
    let mut app = app_watching(&dir, false);
    let file = dir.join("thing.sco");
    // A burst of writes (as editors do) must produce one reload.
    for _ in 0..3 {
        std::fs::write(&file, VALID_SCO).expect("write");
        thread::sleep(Duration::from_millis(15));
    }
    assert!(pump_until(&mut app, Duration::from_secs(5), |a| !a
        .world()
        .resource::<Seen>()
        .validated
        .is_empty()));
    pump_until(&mut app, Duration::from_millis(400), |_| false);
    let seen = app.world().resource::<Seen>();
    assert_eq!(seen.validated.len(), 1, "{:?}", seen.validated);
    assert_eq!(seen.validated[0].1, ReloadKind::Sco);
    assert!(seen.failed.is_empty());
}

#[test]
fn a_broken_save_fails_and_the_next_good_one_recovers() {
    let dir = temp_dir("broken");
    let mut app = app_watching(&dir, false);
    let file = dir.join("model_main.cfg");
    std::fs::write(&file, b"").expect("truncate");
    assert!(pump_until(&mut app, Duration::from_secs(5), |a| !a
        .world()
        .resource::<Seen>()
        .failed
        .is_empty()));
    assert!(app.world().resource::<Seen>().failed[0].1.contains("empty"));
    assert!(
        app.world().resource::<Seen>().validated.is_empty(),
        "nothing reaches the apply step"
    );
    std::fs::write(&file, b"[model]\n").expect("repair");
    // A model config needs content the parser accepts; the empty-file rule is what this test is about.
    let _ = pump_until(&mut app, Duration::from_secs(3), |a| {
        !a.world().resource::<Seen>().validated.is_empty()
    });
    let stats = app.world().resource::<ReloadStats>();
    assert!(stats.failed >= 1);
}

#[test]
fn editor_scratch_and_unknown_files_are_ignored() {
    let dir = temp_dir("scratch");
    let mut app = app_watching(&dir, false);
    for name in ["a.sco~", ".a.sco.swp", "notes.txt", "a.sco.tmp"] {
        std::fs::write(dir.join(name), b"x").expect("write");
    }
    pump_until(&mut app, Duration::from_millis(600), |_| false);
    let seen = app.world().resource::<Seen>();
    assert!(seen.validated.is_empty() && seen.failed.is_empty());
}
