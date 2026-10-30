//! Q-SB-01 probe: renders a test scene into two image targets (black and white clear colours)
//! with no window at all, reads both back with `Readback` and prints the matting result.
//!
//! `cargo run -p opts-benchmark --example offscreen_probe -- <out-dir>`

use std::sync::{Arc, Mutex};

use bevy::app::SubApps;
use bevy::asset::RenderAssetUsages;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::core_pipeline::tonemapping::DebandDither;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{PollType, TextureFormat, TextureUsages};
use bevy::render::renderer::RenderDevice;
use bevy::render::RenderPlugin;
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;

type Shared = Arc<Mutex<Vec<(usize, Vec<u8>)>>>;

/// Spawns the two cameras (black and white clear colour, each with an image target) and the
/// test scene; returns the two image targets.
fn spawn_scene(world: &mut World) -> Vec<Handle<Image>> {
    let mut targets = Vec::new();
    for clear in [Color::BLACK, Color::WHITE] {
        let mut img = Image::new_target_texture(512, 512, TextureFormat::Rgba8UnormSrgb, None);
        img.asset_usage = RenderAssetUsages::RENDER_WORLD;
        img.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let h = world.resource_mut::<Assets<Image>>().add(img);
        world.spawn((
            Camera3d::default(),
            opts_bevy::MainCamera,
            Camera {
                clear_color: ClearColorConfig::Custom(clear),
                ..default()
            },
            RenderTarget::from(h.clone()),
            Msaa::Off,
            Tonemapping::None,
            DebandDither::Disabled,
            Transform::from_xyz(4.0, 4.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));
        targets.push(h);
    }
    let cube = world
        .resource_mut::<Assets<Mesh>>()
        .add(Cuboid::new(2.0, 2.0, 2.0));
    let quad = world
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(2.0, 2.0));
    let grey = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(Color::srgb(0.6, 0.6, 0.6));
    let magenta = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 1.0),
            unlit: true,
            ..default()
        });
    world.spawn((Mesh3d(cube), MeshMaterial3d(grey)));
    world.spawn((
        Mesh3d(quad),
        MeshMaterial3d(magenta),
        Transform::from_xyz(-2.5, 0.0, 0.0),
    ));
    world.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(-3.0, 5.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    targets
}

/// Runs six frames, attaching the readbacks on the third; returns the frame times.
fn run_frames(
    apps: &mut SubApps,
    targets: &[Handle<Image>],
    shared: &Shared,
) -> Vec<std::time::Duration> {
    let mut times = Vec::new();
    for frame in 0..6 {
        let t = std::time::Instant::now();
        if frame == 2 {
            let world = apps.main.world_mut();
            for (i, h) in targets.iter().enumerate() {
                let sink = shared.clone();
                world.spawn(Readback::texture(h.clone())).observe(
                    move |ev: On<ReadbackComplete>| {
                        if let Ok(mut v) = sink.lock() {
                            v.push((i, ev.data.clone()));
                        }
                    },
                );
            }
        }
        apps.update();
        let device = apps.main.world().resource::<RenderDevice>();
        let _ = device.wgpu_device().poll(PollType::Wait {
            submission_index: None,
            timeout: None,
        });
        times.push(t.elapsed());
    }
    times
}

/// Prints the matting result of the black and white readbacks and saves them as PNGs.
fn report(by_index: &[Option<Vec<u8>>], out: &str) {
    let (Some(black), Some(white)) = (&by_index[0], &by_index[1]) else {
        println!("missing readback");
        return;
    };
    let (mut object, mut magenta_px) = (0, 0);
    for (b, w) in black.chunks_exact(4).zip(white.chunks_exact(4)) {
        let d = (0..3).map(|c| b[c].abs_diff(w[c])).max().unwrap_or(0);
        if d < 128 {
            object += 1;
            if b[0] == 255 && b[1] == 0 && b[2] == 255 {
                magenta_px += 1;
            }
        }
    }
    println!(
        "bytes {} object px {object}, exact magenta px {magenta_px}",
        black.len()
    );
    for (name, data) in [("black", black), ("white", white)] {
        let img = image::RgbaImage::from_raw(512, 512, data.clone());
        if let Some(img) = img {
            let _ = img.save(format!("{out}/probe_{name}.png"));
        }
    }
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let t0 = std::time::Instant::now();
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
    );
    app.finish();
    app.cleanup();
    let mut apps: SubApps = std::mem::take(app.sub_apps_mut());
    println!("app built in {:?}", t0.elapsed());

    let targets = spawn_scene(apps.main.world_mut());

    let shared: Shared = Arc::default();
    let times = run_frames(&mut apps, &targets, &shared);
    println!("frame times {times:?}");
    let got = shared.lock().map(|v| v.clone()).unwrap_or_default();
    println!("readbacks received: {}", got.len());
    let mut by_index: Vec<Option<Vec<u8>>> = vec![None, None];
    for (i, d) in got {
        by_index[i].get_or_insert(d);
    }
    report(&by_index, &out);
}
