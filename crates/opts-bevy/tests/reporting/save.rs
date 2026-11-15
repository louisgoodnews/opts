//! Saving a report: the report button writes both files, a capture that never arrives aborts, and the file round trip.

use super::*;

#[test]
fn report_button_saves_both_files_and_closes() {
    let dir = temp_dir("flow");
    std::env::set_var(opts_bevy::reporting::REPORTS_DIR_ENV, &dir);
    let mut app = app();
    std::env::remove_var(opts_bevy::reporting::REPORTS_DIR_ENV);
    press_f1(&mut app);
    deliver_frame(&mut app);
    let btn = app
        .world_mut()
        .query_filtered::<Entity, With<opts_bevy::reporting::modal::ReportButton>>()
        .single(app.world())
        .unwrap();
    app.world_mut().write_message(ButtonClicked(btn));
    for _ in 0..400 {
        app.update();
        if state(&app) == ReportModalState::Inactive {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(state(&app), ReportModalState::Inactive);
    let names: Vec<String> = std::fs::read_dir(&dir)
        .expect("reports dir")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().any(|n| n.ends_with(".png")), "{names:?}");
    assert!(names.iter().any(|n| n.ends_with(".report")), "{names:?}");
    let toasts = app
        .world_mut()
        .query::<&M3Toast>()
        .iter(app.world())
        .count();
    assert_eq!(toasts, 1, "success toast shown");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_capture_that_never_arrives_aborts_cleanly() {
    let mut app = app();
    press_f1(&mut app);
    assert_eq!(state(&app), ReportModalState::Capturing);
    app.world_mut()
        .resource_mut::<opts_bevy::reporting::ReportModalData>()
        .capture_started_at = std::time::Instant::now()
        - opts_bevy::reporting::capture::CAPTURE_TIMEOUT
        - std::time::Duration::from_secs(1);
    app.update();
    app.update();
    assert_eq!(state(&app), ReportModalState::Inactive);
    assert!(!app.world().resource::<Time<Virtual>>().is_paused());
    assert!(!app.world().resource::<ModalActive>().0);
}
#[test]
fn umlauts_symbols_and_emoji_survive_the_file_round_trip() {
    let dir = temp_dir("utf8");
    let description = "Größe ä ö ü ß € ✓ ▼ 😀 – „Zitat“\r\nZeile 2\n";
    let body = format_report(&meta(), description);
    let saved = save_report(&dir, "2026-10-10_14-30-52", b"\x89PNG", &body).expect("saved");
    let bytes = std::fs::read(&saved.report).expect("read back");
    assert!(!bytes.starts_with(&[0xEF, 0xBB, 0xBF]), "no BOM");
    assert!(!bytes.contains(&b'\r'), "\\n line endings only");
    let text = String::from_utf8(bytes).expect("valid UTF-8");
    assert!(text.contains("Größe ä ö ü ß € ✓ ▼ 😀 – „Zitat“\nZeile 2\n"));
    let _ = std::fs::remove_dir_all(dir);
}
