//! Reporting (`request_report`): formatting, file saving, and the headless state machine / pause behaviour.

use bevy::asset::RenderAssetUsages;
use bevy::ecs::system::RunSystemOnce;
use bevy::input::InputPlugin;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::state::app::StatesPlugin;
use bevy::window::PrimaryWindow;
use opts_bevy::reporting::capture::CaptureChannel;
use opts_bevy::reporting::{
    format_report, not_modal_active, save_report, ModalActive, ReportContext, ReportMeta,
    ReportModalState, ReportingPlugin,
};
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::prelude::*;

fn meta() -> ReportMeta {
    ReportMeta::from_context(
        &ReportContext {
            version: Some("0.1.0-alpha".into()),
            git: Some("abc1234".into()),
            map: Some("Berlin-Spandau".into()),
            camera: Some("3 External".into()),
            vehicle: None,
            hardware: None,
        },
        "2026-10-10T14:30:52+02:00".into(),
        Some("TextArea"),
    )
}

#[test]
fn report_text_matches_the_specified_layout() {
    let text = format_report(&meta(), "Zeile 1\r\nZeile 2 – ä ö ü ß ✓\n\nEnde");
    let expected = "OPTS Report\n===========\n\n\
Timestamp:   2026-10-10T14:30:52+02:00\n\
Version:     0.1.0-alpha\n\
Git:         abc1234\n\
Platform:    linux\n\
Map:         Berlin-Spandau\n\
Camera:      3 External\n\
Vehicle:     (none)\n\
Focused UI:  TextArea\n\n\
--- Description ---\n\nZeile 1\nZeile 2 – ä ö ü ß ✓\n\nEnde\n";
    assert_eq!(text.replace(std::env::consts::OS, "linux"), expected);
    assert!(!text.contains('\r'));
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("opts-report-test-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn same_second_reports_get_numeric_suffixes_and_are_never_overwritten() {
    let dir = temp_dir("collide");
    let a = save_report(&dir, "2026-10-10_14-30-52", b"png-a", "a").expect("first");
    let b = save_report(&dir, "2026-10-10_14-30-52", b"png-b", "b").expect("second");
    let c = save_report(&dir, "2026-10-10_14-30-52", b"png-c", "c").expect("third");
    assert!(a.report.ends_with("2026-10-10_14-30-52.report"));
    assert!(b.report.ends_with("2026-10-10_14-30-52-1.report"));
    assert!(c.png.ends_with("2026-10-10_14-30-52-2.png"));
    assert_eq!(std::fs::read(&a.png).expect("a png"), b"png-a");
    assert_eq!(std::fs::read_to_string(&b.report).expect("b report"), "b");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_stray_png_without_report_is_skipped_too() {
    let dir = temp_dir("stray");
    std::fs::create_dir_all(&dir).expect("dir");
    std::fs::write(dir.join("t.png"), b"old").expect("stray");
    let saved = save_report(&dir, "t", b"new", "r").expect("saved");
    assert!(saved.png.ends_with("t-1.png"));
    assert_eq!(std::fs::read(dir.join("t.png")).expect("old"), b"old");
    assert!(
        !dir.join("t.report").exists(),
        "no orphan report may be left behind"
    );
    let _ = std::fs::remove_dir_all(dir);
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        InputPlugin,
        AssetPlugin {
            watch_for_changes_override: Some(false),
            ..default()
        },
        OptsGuiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_asset::<Mesh>()
    .add_plugins(ReportingPlugin);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.update();
    app
}

fn state(app: &App) -> ReportModalState {
    *app.world().resource::<State<ReportModalState>>().get()
}

/// Asks for a report the way the pause menu, Help and the session summary do.
fn press_f1(app: &mut App) {
    app.world_mut()
        .run_system_once(
            |time: Res<Time<Virtual>>,
             mut data: ResMut<opts_bevy::reporting::ReportModalData>,
             state: Res<State<ReportModalState>>,
             mut next: ResMut<NextState<ReportModalState>>| {
                opts_bevy::reporting::request_report(&time, &mut data, &state, &mut next);
            },
        )
        .expect("request");
    app.update();
    app.update();
}

fn deliver_frame(app: &mut App) {
    let img = Image::new_fill(
        Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[10, 20, 30, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    app.world()
        .resource::<CaptureChannel>()
        .sender()
        .send(img)
        .expect("send");
    for _ in 0..200 {
        app.update();
        if state(app) == ReportModalState::Open {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("modal never opened, state {:?}", state(app));
}

#[path = "reporting/pause.rs"]
mod pause;
#[path = "reporting/save.rs"]
mod save;
