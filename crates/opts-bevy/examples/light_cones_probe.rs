//! Light-cone probe: renders the real cone path (`LightConePlugin`, `LightConeRenderPlugin`,
//! `LightCone` entities from a lantern's `[light_enh_2]` values) in a small night street and
//! prints, per scenario, the mean and peak luminance of the frame (post-tonemapping sRGB with
//! `Tonemapping::None`, so the values are what is shown).
//!
//! ```text
//! timeout 60 cargo run --release -p opts-bevy --example light_cones_probe -- --shots /tmp/probe
//! ```
//!
//! Scenarios: `base` (no cones), `road_on/off` (64 lanterns 20 m apart along the road, depth
//! fade on/off), `dense_on/off` (64 lanterns 2.5 m apart: heavy overlap), `inside_*` (the camera
//! inside a lantern's cone, looking along the road and down). The acceptance of the cone work is:
//! the mean luminance of a cone scenario is within 0.04 of `base`, and the peak stays below 0.9.

use std::path::PathBuf;

use bevy::camera::visibility::Visibility;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured};
use bevy::window::WindowResolution;
use opts_bevy::light_cones::{
    lantern_cone_shape, spawn_light_cone, ConeKind, ConeLevel, LightCone, LightConePlugin,
    LightConeRenderPlugin,
};
use opts_bevy::light_quality::{ladder_preset, LightMode, LightQualityInput};
use opts_core::formats::scenery::ScoLight;

#[derive(Clone, Copy, PartialEq)]
enum Layout {
    None,
    Road,
    Dense,
}

#[derive(Clone, Copy, PartialEq)]
enum Eye {
    Street,
    InsideAlong,
    InsideDown,
}

struct Scn {
    label: &'static str,
    layout: Layout,
    fade: bool,
    eye: Eye,
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

#[derive(Component)]
struct Lamp;

/// Seconds a scenario runs before the capture (the depth-fade gate needs 3 s plus 0.3 s).
const SETTLE_S: f32 = 4.5;

#[path = "light_cones_probe/capture.rs"]
mod capture;
#[path = "light_cones_probe/scene.rs"]
mod scene;
use capture::*;
use scene::*;

fn scn(label: &'static str, layout: Layout, fade: bool, eye: Eye) -> Scn {
    Scn {
        label,
        layout,
        fade,
        eye,
    }
}

fn scenarios() -> Vec<Scn> {
    vec![
        scn("base", Layout::None, true, Eye::Street),
        scn("road_on", Layout::Road, true, Eye::Street),
        scn("road_off", Layout::Road, false, Eye::Street),
        scn("dense_on", Layout::Dense, true, Eye::Street),
        scn("dense_off", Layout::Dense, false, Eye::Street),
        scn("inside_along_on", Layout::Road, true, Eye::InsideAlong),
        scn("inside_down_on", Layout::Road, true, Eye::InsideDown),
        scn("inside_along_off", Layout::Road, false, Eye::InsideAlong),
    ]
}

fn main() {
    let shots = std::env::args()
        .position(|a| a == "--shots")
        .and_then(|i| std::env::args().nth(i + 1))
        .map(PathBuf::from);
    let scenarios = scenarios();
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "light_cones_probe".into(),
            resolution: WindowResolution::new(480, 270).with_scale_factor_override(1.0),
            present_mode: bevy::window::PresentMode::AutoNoVsync,
            ..default()
        }),
        ..default()
    }))
    .add_plugins((LightConePlugin, LightConeRenderPlugin))
    .insert_resource(ClearColor(Color::srgb(0.01, 0.012, 0.02)))
    .insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 40.0,
        ..default()
    })
    .insert_resource(Probe {
        scenarios,
        idx: 0,
        started_s: -1.0,
        shot_taken: false,
        shots,
        lines: Vec::new(),
    })
    .add_systems(Startup, setup)
    .add_systems(Update, drive);
    app.run();
}
