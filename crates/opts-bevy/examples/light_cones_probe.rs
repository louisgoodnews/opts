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

fn lantern() -> ScoLight {
    // `neonlight_M_whip_S.sco`: an arm over the road, pointing down and slightly inwards.
    ScoLight {
        position: Vec3::new(-2.965, 0.0, 7.17),
        direction: Vec3::new(-0.3817, 0.0, -1.5344).normalize(),
        color: [230.0, 230.0, 255.0],
        size_m: 2.0,
        variable: "NightlightA".into(),
        factor: 0.8,
        omni: false,
        cone_inner_deg: 120.0,
        cone_outer_deg: 200.0,
        time_const_s: 0.2,
        volumetric_cone: true,
        bitmap: String::new(),
        after_mesh: None,
        lod_threshold: None,
    }
}

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
        Mesh3d(meshes.add(Plane3d::default().mesh().size(400.0, 800.0))),
        MeshMaterial3d(ground.clone()),
        Transform::from_xyz(0.0, 0.0, -300.0),
    ));
    // Facades on both sides of the road: where the depth fade matters.
    for side in [-1.0f32, 1.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 12.0, 700.0))),
            MeshMaterial3d(ground.clone()),
            Transform::from_xyz(side * 9.0, 6.0, -300.0),
        ));
    }
    commands.spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        Tonemapping::None,
        Transform::from_xyz(0.0, 1.7, 5.0).looking_to(Vec3::NEG_Z, Vec3::Y),
    ));
}

/// Sets the current scenario up: despawns the old lamps, applies the quality settings, spawns the
/// new lamps and places the camera.
fn setup_scenario(
    commands: &mut Commands,
    probe: &mut Probe,
    lamps: &Query<Entity, With<Lamp>>,
    camera: &mut Query<&mut Transform, With<opts_bevy::MainCamera>>,
) {
    let idx = probe.idx;
    for e in lamps.iter() {
        commands.entity(e).despawn();
    }
    let (layout, fade, eye) = {
        let s = &probe.scenarios[idx];
        (s.layout, s.fade, s.eye)
    };
    let mut q = ladder_preset(0);
    q.cone_depth_fade = fade;
    q.headlight_cones = false;
    commands.insert_resource(LightQualityInput {
        mode: LightMode::Custom,
        custom: q,
    });
    let shape = lantern_cone_shape(&lantern()).expect("lantern cone");
    let (spacing, count) = match layout {
        Layout::None => (0.0, 0),
        Layout::Road => (20.0, 64),
        Layout::Dense => (2.5, 64),
    };
    for i in 0..count {
        let root = commands
            .spawn((
                Transform::from_xyz(4.0, 0.0, -10.0 - spacing * i as f32),
                Visibility::Inherited,
                Lamp,
            ))
            .id();
        spawn_light_cone(
            commands,
            root,
            LightCone {
                kind: ConeKind::Lantern,
                shape,
                level: ConeLevel::Fixed(0.8),
            },
        );
    }
    if let Ok(mut t) = camera.single_mut() {
        let apex = Vec3::new(4.0, 0.0, -10.0) + shape.apex;
        *t = match eye {
            Eye::Street => Transform::from_xyz(0.0, 1.7, 5.0).looking_to(Vec3::NEG_Z, Vec3::Y),
            // Just below the lamp head, inside the cone.
            Eye::InsideAlong => Transform::from_translation(apex + Vec3::new(-0.2, -1.0, 0.0))
                .looking_to(Vec3::NEG_Z, Vec3::Y),
            Eye::InsideDown => Transform::from_translation(apex + Vec3::new(-0.2, -1.0, 0.0))
                .looking_to(Vec3::new(0.0, -1.0, -0.3), Vec3::NEG_Z),
        };
    }
}

/// Mean, peak and centre luminance of a captured frame, as one report line.
fn frame_line(label: &str, img: &image::RgbaImage) -> String {
    let (w, h) = img.dimensions();
    let luma = |p: &image::Rgba<u8>| {
        (0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2])) / 255.0
    };
    let (mut sum, mut peak, mut peak_ch) = (0.0f32, 0.0f32, 0.0f32);
    for p in img.pixels() {
        let l = luma(p);
        sum += l;
        peak = peak.max(l);
        peak_ch = peak_ch.max(f32::from(p[0].max(p[1]).max(p[2])) / 255.0);
    }
    let mut centre = 0.0f32;
    let mut n = 0.0f32;
    for y in (h / 2 - 20)..(h / 2 + 20) {
        for x in (w / 2 - 20)..(w / 2 + 20) {
            centre += luma(img.get_pixel(x, y));
            n += 1.0;
        }
    }
    format!(
        "{label}: mean {:.4} peak_luma {peak:.3} peak_channel {peak_ch:.3} centre_mean {:.4}",
        sum / (w * h) as f32,
        centre / n
    )
}

fn drive(
    mut commands: Commands,
    time: Res<Time>,
    mut probe: ResMut<Probe>,
    lamps: Query<Entity, With<Lamp>>,
    mut camera: Query<&mut Transform, With<opts_bevy::MainCamera>>,
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
        setup_scenario(&mut commands, &mut probe, &lamps, &mut camera);
        probe.started_s = now;
        probe.shot_taken = false;
        return;
    }
    if now - probe.started_s >= SETTLE_S && !probe.shot_taken {
        probe.shot_taken = true;
        let label = probe.scenarios[idx].label;
        let mut shot = commands.spawn(Screenshot::primary_window());
        if let Some(dir) = probe.shots.clone() {
            let _ = std::fs::create_dir_all(&dir);
            shot.observe(save_to_disk(dir.join(format!("{label}.png"))));
        }
        shot.observe(
            move |cap: On<ScreenshotCaptured>, mut probe: ResMut<Probe>| {
                let img = cap.image.clone().try_into_dynamic().expect("image").to_rgba8();
                probe.lines.push(frame_line(label, &img));
                probe.idx += 1;
                probe.started_s = -1.0;
            },
        );
    }
}
