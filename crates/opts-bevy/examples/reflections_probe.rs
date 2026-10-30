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

use std::path::PathBuf;

use bevy::camera::Hdr;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured};
use bevy::window::WindowResolution;
use opts_bevy::glass::{register_surface, SurfaceKind};
use opts_bevy::light_quality::{ladder_preset, LightMode, LightQualityInput, LightQualityPlugin};
use opts_bevy::mirrors::MirrorStats;
use opts_bevy::reflections::ReflectionsPlugin;
use opts_bevy::sky_cube::{SkyCubeInput, SkyCubeState};
use opts_bevy::PlayerVehicle;

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
    .add_systems(Startup, setup)
    .add_systems(Update, drive);
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ground = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.37),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(100.0, 100.0))),
        MeshMaterial3d(ground),
        Transform::default(),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 20_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::default().looking_to(-Vec3::new(0.4, 0.7, 0.3), Vec3::Y),
    ));
    // The cubes the mirror must show: behind the main camera.
    for (x, c) in [
        (0.0, Color::srgb(0.9, 0.1, 0.1)),
        (2.0, Color::srgb(0.1, 0.2, 0.9)),
    ] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.5, 1.5, 1.5))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: c,
                ..default()
            })),
            Transform::from_xyz(x, 0.75, 6.0),
        ));
    }
    // The player vehicle with a window pane and a mirror pane.
    let vehicle = commands
        .spawn((PlayerVehicle, Transform::default(), Visibility::Inherited))
        .id();
    let pane = meshes.add(Rectangle::new(0.7, 0.45));
    let glass_mat = StandardMaterial {
        base_color: Color::srgba(0.35, 0.45, 0.5, 0.25),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.8,
        ..default()
    };
    let glass = materials.add(glass_mat.clone());
    register_surface(&glass, SurfaceKind::Glass, &glass_mat);
    let mirror_mat = StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.5),
        unlit: true,
        ..default()
    };
    let mirror = materials.add(mirror_mat.clone());
    register_surface(&mirror, SurfaceKind::Mirror, &mirror_mat);
    for (at, mat) in [(GLASS_AT, glass), (MIRROR_AT, mirror)] {
        let e = commands
            .spawn((
                Mesh3d(pane.clone()),
                MeshMaterial3d(mat),
                Transform::from_translation(at),
            ))
            .id();
        commands.entity(vehicle).add_child(e);
    }
    commands.spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        Hdr,
        Transform::from_xyz(0.0, 1.5, 0.0).looking_to(Vec3::NEG_Z, Vec3::Y),
    ));
}

/// Applies the current scenario's light-quality settings and starts its settle timer.
fn start_scenario(commands: &mut Commands, probe: &mut Probe, now: f32) {
    let idx = probe.idx;
    let (glass, mirrors) = (probe.scenarios[idx].glass, probe.scenarios[idx].mirrors);
    let mut q = ladder_preset(0);
    q.glass_reflections = glass;
    q.mirror_count = mirrors;
    q.mirror_every = 1;
    commands.insert_resource(LightQualityInput {
        mode: LightMode::Custom,
        custom: q,
    });
    probe.started_s = now;
    probe.shot_taken = false;
}

fn drive(
    mut commands: Commands,
    time: Res<Time>,
    mut probe: ResMut<Probe>,
    stats: Res<MirrorStats>,
    cube: Res<SkyCubeState>,
    camera: Query<(&Camera, &GlobalTransform), With<opts_bevy::MainCamera>>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    if probe.idx >= probe.scenarios.len() {
        for l in &probe.lines {
            println!("{l}");
        }
        exit.write(AppExit::Success);
        return;
    }
    let idx = probe.idx;
    if probe.started_s < 0.0 {
        start_scenario(&mut commands, &mut probe, now);
        return;
    }
    if now - probe.started_s >= SETTLE_S && !probe.shot_taken {
        probe.shot_taken = true;
        let label = probe.scenarios[idx].label;
        let project = |p: Vec3| {
            camera
                .single()
                .ok()
                .and_then(|(c, gt)| c.world_to_viewport(gt, p).ok())
        };
        let (glass_px, mirror_px) = (project(GLASS_AT), project(MIRROR_AT));
        let (cameras, selected, rendering) = (stats.cameras, stats.selected, stats.rendering);
        let (builds, intensity) = (cube.builds, cube.intensity);
        let mut shot = commands.spawn(Screenshot::primary_window());
        if let Some(dir) = probe.shots.clone() {
            let _ = std::fs::create_dir_all(&dir);
            shot.observe(save_to_disk(dir.join(format!("{label}.png"))));
        }
        shot.observe(
            move |cap: On<ScreenshotCaptured>, mut probe: ResMut<Probe>| {
                let img = cap.image.clone().try_into_dynamic().expect("image").to_rgba8();
                let region = |centre: Option<Vec2>| -> String {
                    let Some(c) = centre else {
                        return "off-screen".into();
                    };
                    let (mut r, mut g, mut b, mut n) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
                    for y in (c.y as i32 - 12)..(c.y as i32 + 12) {
                        for x in (c.x as i32 - 20)..(c.x as i32 + 20) {
                            if let Some(p) = img.get_pixel_checked(x.max(0) as u32, y.max(0) as u32)
                            {
                                r += f32::from(p[0]);
                                g += f32::from(p[1]);
                                b += f32::from(p[2]);
                                n += 1.0;
                            }
                        }
                    }
                    format!("rgb {:.0} {:.0} {:.0}", r / n, g / n, b / n)
                };
                probe.lines.push(format!(
                    "{label}: glass {} | mirror {} | ground {} | sky {} | cameras {cameras} selected {selected} rendering {rendering} | cube builds {builds} intensity {intensity:.0}",
                    region(glass_px),
                    region(mirror_px),
                    region(Some(Vec2::new(120.0, 235.0))),
                    region(Some(Vec2::new(120.0, 30.0))),
                ));
                probe.idx += 1;
                probe.started_s = -1.0;
            },
        );
    }
}
