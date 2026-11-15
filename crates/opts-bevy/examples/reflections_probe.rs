//! Reflections probe: renders the real sky-cube, glass and mirror-camera path in a tiny scene
//! (a window pane and a mirror pane in front of an HDR main camera, a red and a blue cube behind
//! it) and prints, per scenario, the mean luminance around the glass and the mirror pane.
//!
//! ```text
//! timeout 60 cargo run --release -p opts-bevy --example reflections_probe -- --shots /tmp/probe
//! ```
//!
//! Scenarios: `off` (nothing), `glass` (sky cube and glass look), `mirror` (one mirror camera,
//! every frame), `both`. The probe's quad has plain UVs (`u` to the viewer's right), the real
//! mirror panes have UVs that run right to left: run it with `OPTS_MIRROR_FLIP=u` to see the
//! mirrored picture (the blue cube, right of the red one behind the camera, then shows on the
//! right). The acceptance: the glass region differs between `off` and `glass`, the
//! mirror region differs between `off` and `mirror` and shows the cubes' colours, and no GPU
//! validation error is logged.

#[path = "reflections_probe/drive.rs"]
mod drive;
#[path = "reflections_probe/scene.rs"]
mod scene;

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use opts_bevy::light_quality::LightQualityPlugin;
use opts_bevy::reflections::ReflectionsPlugin;
use opts_bevy::sky_cube::SkyCubeInput;

struct Scn {
    label: &'static str,
    glass: bool,
    mirrors: u8,
}

#[derive(Resource)]
struct Probe {
    scenarios: Vec<Scn>,
    idx: usize,
    started_s: f32,
    shot_taken: bool,
    shots: Option<PathBuf>,
    lines: Vec<String>,
}

const SETTLE_S: f32 = 3.0;
const GLASS_AT: Vec3 = Vec3::new(-0.45, 1.5, -1.5);
const MIRROR_AT: Vec3 = Vec3::new(0.45, 1.5, -1.5);

fn main() {
    let shots = std::env::args()
        .position(|a| a == "--shots")
        .and_then(|i| std::env::args().nth(i + 1))
        .map(PathBuf::from);
    let scenarios = vec![
        Scn {
            label: "off",
            glass: false,
            mirrors: 0,
        },
        Scn {
            label: "glass",
            glass: true,
            mirrors: 0,
        },
        Scn {
            label: "mirror",
            glass: false,
            mirrors: 1,
        },
        Scn {
            label: "both",
            glass: true,
            mirrors: 2,
        },
    ];
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "reflections_probe".into(),
            resolution: WindowResolution::new(480, 270).with_scale_factor_override(1.0),
            present_mode: bevy::window::PresentMode::AutoNoVsync,
            ..default()
        }),
        ..default()
    }))
    .add_plugins((LightQualityPlugin, ReflectionsPlugin))
    .insert_resource(ClearColor(Color::srgb(0.45, 0.6, 0.85)))
    .insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 700.0,
        ..default()
    })
    .insert_resource(SkyCubeInput {
        horizon: [0.75, 0.82, 0.95],
        zenith: [0.15, 0.32, 0.75],
        sun_dir: Vec3::new(0.4, 0.7, 0.3).normalize(),
        sun_color: [1.0, 0.95, 0.85],
        sun_elevation_deg: 44.0,
        direct: 1.0,
        ambient: 1.0,
        fog: 0.0,
        revision: 1,
    })
    .insert_resource(Probe {
        scenarios,
        idx: 0,
        started_s: -1.0,
        shot_taken: false,
        shots,
        lines: Vec::new(),
    })
    .add_systems(Startup, scene::setup)
    .add_systems(Update, drive::drive);
    app.run();
}
