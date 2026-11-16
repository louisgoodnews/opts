//! Headless check of the weather effects: renders a small scene (layered ground, a few boxes,
//! a camera that is the drive camera for the weather systems) off-screen under several presets and
//! writes one PNG per preset. With `OMSI_PATH` set the install's presets and cloud textures are
//! used, else the built-in presets (no cloud textures).
//!
//! `cargo run -p opts-game --example weather_probe -- <out-dir> [preset ...]`
//!
//! `OPTS_PROBE_HEIGHTS=2,70` renders each preset once per camera height (metres above the flat
//! ground), `OPTS_PROBE_PITCH=-8` sets the camera pitch in degrees (default: a slight upward
//! look as before), to check the ground-fog layer (`Bodennebel`).

use std::sync::{Arc, Mutex};

use bevy::app::SubApps;
use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{PollType, TextureFormat, TextureUsages};
use bevy::render::renderer::RenderDevice;
use bevy::render::RenderPlugin;
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;
use opts_bevy::ground::CpuImage;
use opts_bevy::ground_material::{
    ground_extension, layer_array, mask_array, GroundMaterial, GroundMaterialPlugin, GroundSlot,
};
use opts_bevy::{
    CalendarDate, EnvironmentChange, EnvironmentPlugin, EnvironmentSet, WeatherCatalogue,
};
use opts_game::in_game::types::InGameCamera;
use opts_game::in_game::weather::{clouds, fog, precip, surfaces};

const W: u32 = 960;
const H: u32 = 540;

#[path = "weather_probe/render.rs"]
mod render;
#[path = "weather_probe/scene.rs"]
mod scene;
use render::{probe_view, render_job};
use scene::{build_app, spawn_camera_and_sun, spawn_ground};

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| ".".into());
    let mut presets: Vec<String> = args.collect();
    let mut app = build_app();
    if presets.is_empty() {
        presets = app.world().resource::<WeatherCatalogue>().ids();
    }
    app.finish();
    app.cleanup();
    let mut apps: SubApps = std::mem::take(app.sub_apps_mut());
    let world = apps.main.world_mut();
    let target = spawn_camera_and_sun(world);
    spawn_ground(world);

    let shared: Arc<Mutex<Vec<u8>>> = Arc::default();
    let (heights, pitch) = probe_view();
    let jobs: Vec<(usize, &String, f32)> = presets
        .iter()
        .enumerate()
        .flat_map(|(n, id)| heights.iter().map(move |h| (n, id, *h)))
        .collect();
    for (n, id, height) in jobs {
        render_job(
            &mut apps,
            &shared,
            &target,
            (n, id.as_str(), height),
            pitch,
            &out,
        );
    }
}
