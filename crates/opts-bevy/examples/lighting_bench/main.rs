//! Lighting benchmark: sun god rays (Bevy volumetric fog) and light-cone prototypes.
//!
//! Design note: `docs/src/architecture/lighting-cones.md`. The example renders a synthetic street
//! canyon (about 300 textured boxes, 100 alpha-masked tree quads, 48 map point lights, 6 spot
//! lights, a shadow-casting sun) at 1920x1080 into an offscreen image (or the window with
//! `--window`) and measures frame times and GPU pass times for a *plan* of configurations.
//!
//! ```text
//! cargo run --release -p opts-bevy --example lighting_bench -- \
//!     --plan 'base|scene=sun,fog=0;rays|scene=sun,fog=1,steps=32' --out /tmp/x.jsonl --shots /tmp/shots
//! ```
//!
//! Each plan item is `label|key=value,key=value`; `--base` holds keys common to all items.
//! Keys (defaults in brackets):
//!
//! * `scene` (sun | night) - day scene with the shadow-casting sun, or night (sun off, lamps on)
//! * `elev` [10] sun elevation in degrees, `look` [toward] | away, `shadow` [2048] sun shadow map
//! * `fog` [0], `steps` [64], `jitter` [0], `density` [0.04], `ambient` [0], `vol` [big | small]
//! * `msaa` [4], `hdr` [0], `prepass` [0] (depth prepass on the camera)
//! * `tech` [none] | mesh | march | vspot; `n` [0] cones; `fade` [0] depth fade / depth clip
//!   (needs the prepass), `msteps` [8] march steps
//! * `spotsh` [0] real spot light shadows (of the 6 spots), `vspots` [0] VolumetricLight spots
//! * `frames` [600], `warmup` [120], `shot` [0] save a screenshot after warm-up

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::PathBuf;

use bevy::asset::{load_internal_asset, RenderAssetUsages};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{Hdr, RenderTarget};
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::diagnostic::DiagnosticsStore;
use bevy::light::GeneratedEnvironmentMapLight;
use bevy::light::{
    CascadeShadowConfigBuilder, DirectionalLightShadowMap, FogVolume, GlobalAmbientLight,
    NotShadowCaster, NotShadowReceiver, VolumetricFog, VolumetricLight,
};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::mesh::{Indices, MeshTag, PrimitiveTopology};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::diagnostic::RenderDiagnosticsPlugin;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    TextureDimension, TextureFormat, TextureUsages,
};
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::render::view::Msaa;
use bevy::shader::ShaderRef;
use bevy::window::{PresentMode, WindowResolution};

mod cone_material;
use cone_material::{ConeMaterial, SHADER};

const W: u32 = 1920;
const H: u32 = 1080;

// ---------------------------------------------------------------------------------------------
// Cone material
// ---------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------
// Plan
// ---------------------------------------------------------------------------------------------

mod config;
use config::{arg, parse_kv, Cfg};

#[derive(Resource)]
struct Bench {
    plan: Vec<Cfg>,
    idx: usize,
    /// Frames since the current item was applied.
    frame: u32,
    ms: Vec<f32>,
    gpu: BTreeMap<String, Vec<f64>>,
    out: Option<PathBuf>,
    shots: Option<PathBuf>,
    started: std::time::Instant,
    item_start: std::time::Instant,
    shot_done: bool,
    max_seconds: f32,
    load: String,
}

#[derive(Component)]
struct Cone;
#[derive(Component)]
struct Sun;
#[derive(Component)]
struct NightLamp(usize);
#[derive(Component)]
struct HeadSpot(usize);
#[derive(Component)]
struct Cam;
#[derive(Component)]
struct Fog;
#[derive(Component)]
struct Mirror;
#[derive(Resource, Default)]
struct MirrorEvery(u32);

#[derive(Resource)]
struct Assets3 {
    cone: Handle<Mesh>,
    target: Option<Handle<Image>>,
    quad: Handle<Mesh>,
}

fn main() {
    let base = parse_kv(&arg("--base").unwrap_or_default());
    let plan: Vec<Cfg> = arg("--plan")
        .unwrap_or_else(|| "default|scene=sun".into())
        .split(';')
        .filter(|s| !s.trim().is_empty())
        .map(|item| {
            let (label, rest) = item.split_once('|').unwrap_or((item, ""));
            let mut kv = base.clone();
            kv.extend(parse_kv(rest));
            Cfg {
                label: label.trim().to_string(),
                kv,
            }
        })
        .collect();
    let window_mode = std::env::args().any(|a| a == "--window");
    let vsync = std::env::args().any(|a| a == "--vsync");

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "lighting_bench".into(),
            resolution: if window_mode {
                WindowResolution::new(W, H).with_scale_factor_override(1.0)
            } else {
                WindowResolution::new(480, 270).with_scale_factor_override(1.0)
            },
            present_mode: if vsync {
                PresentMode::AutoVsync
            } else {
                PresentMode::AutoNoVsync
            },
            ..default()
        }),
        ..default()
    }));
    if std::env::var_os("LB_NO_GPU_DIAG").is_none() {
        app.add_plugins(RenderDiagnosticsPlugin);
    }
    load_internal_asset!(app, SHADER, "lighting_bench.wesl", Shader::from_wesl);
    app.add_plugins(MaterialPlugin::<ConeMaterial>::default());
    app.insert_resource(Bench {
        plan,
        idx: 0,
        frame: 0,
        ms: Vec::new(),
        gpu: BTreeMap::new(),
        out: arg("--out").map(PathBuf::from),
        shots: arg("--shots").map(PathBuf::from),
        started: std::time::Instant::now(),
        item_start: std::time::Instant::now(),
        shot_done: false,
        max_seconds: arg("--max-seconds")
            .and_then(|v| v.parse().ok())
            .unwrap_or(55.0),
        load: std::fs::read_to_string("/proc/loadavg")
            .unwrap_or_default()
            .split_whitespace()
            .take(3)
            .collect::<Vec<_>>()
            .join(" "),
    });
    app.insert_resource(ClearColor(Color::srgb(0.52, 0.70, 0.95)));
    app.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 700.0,
        ..default()
    });
    app.insert_resource(DirectionalLightShadowMap { size: 2048 });
    app.insert_resource(WindowModeFlag(window_mode));
    app.add_systems(Startup, setup);
    app.init_resource::<MirrorEvery>();
    app.add_systems(Update, drive);
    app.add_systems(Update, mirror_toggle.after(drive));
    app.run();
}

#[derive(Resource)]
struct WindowModeFlag(bool);

// ---------------------------------------------------------------------------------------------
// Scene
// ---------------------------------------------------------------------------------------------

mod procedural;

mod setup;
use setup::setup;

// ---------------------------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------------------------

fn pct(sorted: &[f32], p: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((sorted.len() as f32 - 1.0) * p).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

mod drive;
use drive::drive;

mod apply;
use apply::mirror_toggle;
