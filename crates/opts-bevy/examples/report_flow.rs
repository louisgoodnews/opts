//! End-to-end check of the F1 report flow with a real window and GPU.
//!
//! A cube spins on virtual time. A script presses F1, waits for the modal, types a German text
//! (umlauts, ✓), checks that virtual time stood still while the modal was open, clicks Report and
//! verifies the written files. Run: `OPTS_REPORTS_DIR=/tmp/reports cargo run -p opts-bevy --example report_flow`.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use opts_bevy::reporting::modal::ReportButton;
use opts_bevy::reporting::{ReportContext, ReportModalState, ReportingPlugin, ReportsDir};
use opts_gui::prelude::*;

#[derive(Resource, Default)]
struct Script {
    frame: u32,
    open_frame: Option<u32>,
    time_at_open: f64,
    clicked: bool,
    done_frame: Option<u32>,
}

#[derive(Component)]
struct Spinner;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: bevy::window::WindowResolution::new(900, 600),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((OptsGuiPlugin, ReportingPlugin))
        .init_resource::<Script>()
        .insert_resource(ReportContext {
            version: Some("test".into()),
            map: Some("Berlin-Spandau".into()),
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (spin, script))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.5, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn(DirectionalLight::default());
    commands.spawn((
        (
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(mats.add(Color::srgb(0.8, 0.3, 0.2))),
        ),
        Spinner,
    ));
}

fn spin(time: Res<Time>, mut q: Query<&mut Transform, With<Spinner>>) {
    for mut t in &mut q {
        t.rotate_y(time.delta_secs());
    }
}

fn key(window: Entity, code: KeyCode, logical: Key) -> KeyboardInput {
    KeyboardInput {
        key_code: code,
        logical_key: logical,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    }
}

#[allow(clippy::too_many_arguments)]
fn script(
    mut s: ResMut<Script>,
    state: Res<State<ReportModalState>>,
    vtime: Res<Time<Virtual>>,
    window: Query<Entity, With<PrimaryWindow>>,
    report_btn: Query<Entity, With<ReportButton>>,
    dir: Res<ReportsDir>,
    mut keys: MessageWriter<KeyboardInput>,
    mut clicks: MessageWriter<ButtonClicked>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
) {
    s.frame += 1;
    let Ok(w) = window.single() else { return };
    if s.frame == 40 {
        keys.write(key(w, KeyCode::F1, Key::F1));
    }
    if *state.get() == ReportModalState::Open && s.open_frame.is_none() {
        s.open_frame = Some(s.frame);
        s.time_at_open = vtime.elapsed_secs_f64();
        info!(
            "modal open at frame {}, virtual time {:.3}, paused {}",
            s.frame,
            s.time_at_open,
            vtime.is_paused()
        );
    }
    if let Some(of) = s.open_frame {
        let f = s.frame - of;
        if f == 10 {
            for ch in ["G", "r", "ö", "ß", "e", " ", "✓"] {
                keys.write(key(w, KeyCode::KeyA, Key::Character(ch.into())));
            }
            keys.write(key(w, KeyCode::Enter, Key::Enter));
            for ch in ["Z", "w", "e", "i", "t", "e"] {
                keys.write(key(w, KeyCode::KeyA, Key::Character(ch.into())));
            }
        }
        if f == 40 {
            if let Ok(path) = std::env::var("OPTS_EXAMPLE_MODAL_SHOT") {
                commands
                    .spawn(bevy::render::view::screenshot::Screenshot::window(w))
                    .observe(bevy::render::view::screenshot::save_to_disk(path));
            }
        }
        if f == 60 {
            let drift = vtime.elapsed_secs_f64() - s.time_at_open;
            info!("virtual time drift while modal open: {drift:.4} s (must be 0)");
            if drift > 1e-6 {
                error!("SIMULATION DID NOT PAUSE");
            }
            if let Ok(b) = report_btn.single() {
                clicks.write(ButtonClicked(b));
                s.clicked = true;
            }
        }
    }
    if s.clicked && *state.get() == ReportModalState::Inactive && s.done_frame.is_none() {
        s.done_frame = Some(s.frame);
        info!("modal closed; paused now: {}", vtime.is_paused());
        if let Some(d) = &dir.0 {
            for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
                info!("saved: {}", e.path().display());
            }
        }
    }
    if s.done_frame.is_some_and(|d| s.frame > d + 20) {
        exit.write(AppExit::Success);
    }
}
