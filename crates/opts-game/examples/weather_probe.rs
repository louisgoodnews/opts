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

fn solid(r: u8, g: u8, b: u8) -> CpuImage {
    CpuImage {
        width: 4,
        height: 4,
        rgba: [r, g, b, 255].repeat(16),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().unwrap_or_else(|| ".".into());
    let mut presets: Vec<String> = args.collect();
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .disable::<WinitPlugin>(),
    )
    .add_plugins((GroundMaterialPlugin, EnvironmentPlugin))
    .init_resource::<opts_bevy::reflections::Wetness>()
    .init_resource::<opts_bevy::weather::GroundFogView>()
    .init_resource::<clouds::CloudState>()
    .init_resource::<precip::PrecipState>()
    .add_systems(
        Update,
        (
            fog::measure_fog_view,
            fog::update_fog.after(fog::measure_fog_view),
            clouds::update_clouds,
            precip::update_precipitation,
            (surfaces::drive_wetness, surfaces::update_surfaces).chain(),
        )
            .after(EnvironmentSet),
    );
    if let Some(root) = std::env::var_os("OMSI_PATH") {
        if let Some(c) = WeatherCatalogue::read(std::path::Path::new(&root)) {
            app.insert_resource(c);
        }
    }
    if presets.is_empty() {
        presets = app.world().resource::<WeatherCatalogue>().ids();
    }
    app.finish();
    app.cleanup();
    let mut apps: SubApps = std::mem::take(app.sub_apps_mut());
    let world = apps.main.world_mut();

    let mut img = Image::new_target_texture(W, H, TextureFormat::Rgba8UnormSrgb, None);
    img.asset_usage = RenderAssetUsages::RENDER_WORLD;
    img.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target = world.resource_mut::<Assets<Image>>().add(img);
    world.insert_resource(ClearColor(Color::srgb(0.55, 0.7, 0.9)));
    world.spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        RenderTarget::from(target.clone()),
        Transform::from_xyz(0.0, 2.0, 0.0).looking_at(Vec3::new(0.0, 6.0, -60.0), Vec3::Y),
        InGameCamera,
    ));
    world.spawn((
        DirectionalLight {
            illuminance: 15_000.0,
            ..default()
        },
        Transform::from_xyz(-3.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // Layered ground (one grey-green layer) so the ground shader, with its wet and snow code,
    // is the one that runs.
    let layers = {
        let l = solid(90, 110, 70);
        world
            .resource_mut::<Assets<Image>>()
            .add(layer_array(&[&l], 4))
    };
    let details = {
        let l = solid(255, 255, 255);
        world
            .resource_mut::<Assets<Image>>()
            .add(layer_array(&[&l], 4))
    };
    let masks = world
        .resource_mut::<Assets<Image>>()
        .add(mask_array(&[None], 4));
    let ground = world
        .resource_mut::<Assets<GroundMaterial>>()
        .add(GroundMaterial {
            base: StandardMaterial {
                perceptual_roughness: 0.9,
                ..default()
            },
            extension: ground_extension(
                layers,
                details,
                masks,
                &[GroundSlot {
                    layer: 0,
                    repeats: 40.0,
                    detail_repeats: 1.0,
                }],
            ),
        });
    let plane = world
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(2000.0, 2000.0));
    world.spawn((
        Mesh3d(plane),
        MeshMaterial3d(ground),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    let cube = world
        .resource_mut::<Assets<Mesh>>()
        .add(Cuboid::new(6.0, 10.0, 6.0));
    let wall = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(Color::srgb(0.75, 0.55, 0.45));
    for (i, z) in [-20.0, -50.0, -100.0, -200.0, -400.0]
        .into_iter()
        .enumerate()
    {
        let x = if i % 2 == 0 { -8.0 } else { 8.0 };
        world.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(wall.clone()),
            Transform::from_xyz(x, 5.0, z),
        ));
    }

    let shared: Arc<Mutex<Vec<u8>>> = Arc::default();
    let step = |apps: &mut SubApps| {
        apps.update();
        let device = apps.main.world().resource::<RenderDevice>();
        let _ = device.wgpu_device().poll(PollType::Wait {
            submission_index: None,
            timeout: None,
        });
    };
    let heights: Vec<f32> = std::env::var("OPTS_PROBE_HEIGHTS")
        .ok()
        .map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect())
        .unwrap_or_default();
    let pitch: Option<f32> = std::env::var("OPTS_PROBE_PITCH")
        .ok()
        .and_then(|v| v.parse().ok());
    let heights = if heights.is_empty() {
        vec![2.0]
    } else {
        heights
    };
    let jobs: Vec<(usize, &String, f32)> = presets
        .iter()
        .enumerate()
        .flat_map(|(n, id)| heights.iter().map(move |h| (n, id, *h)))
        .collect();
    for (n, id, height) in jobs {
        {
            let mut q = apps
                .main
                .world_mut()
                .query_filtered::<&mut Transform, With<InGameCamera>>();
            let mut t = q.single_mut(apps.main.world_mut()).unwrap();
            let target = match pitch {
                Some(p) => Vec3::new(0.0, height + 60.0 * p.to_radians().tan(), -60.0),
                None => Vec3::new(0.0, height + 4.0, -60.0),
            };
            *t = Transform::from_xyz(0.0, height, 0.0).looking_at(target, Vec3::Y);
        }
        let hour = if id.to_lowercase().contains("daemmer") {
            19.0
        } else {
            12.0
        };
        apps.main.world_mut().write_message(EnvironmentChange {
            date: Some(CalendarDate::new(2026, 6, 29)),
            seconds_of_day: Some(hour * 3600.0),
            weather: Some(id.clone()),
            ..default()
        });
        for _ in 0..40 {
            step(&mut apps);
        }
        shared.lock().unwrap().clear();
        let sink = shared.clone();
        let e = apps
            .main
            .world_mut()
            .spawn(Readback::texture(target.clone()))
            .observe(move |ev: On<ReadbackComplete>| {
                if let Ok(mut v) = sink.lock() {
                    *v = ev.data.clone();
                }
            })
            .id();
        for _ in 0..4 {
            step(&mut apps);
        }
        apps.main.world_mut().despawn(e);
        let data = shared.lock().unwrap().clone();
        let name: String = id
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        if let Some(img) = image::RgbaImage::from_raw(W, H, data) {
            let path = format!("{out}/{n:02}_{name}_h{height:.0}.png");
            let _ = img.save(&path);
            println!("{id}: {path}");
        } else {
            println!("{id}: no readback");
        }
    }
}
