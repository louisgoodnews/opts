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

const SHADER: Handle<Shader> = bevy::asset::uuid_handle!("6c696768-7462-656e-6368-636f6e657300");
const W: u32 = 1920;
const H: u32 = 1080;

// ---------------------------------------------------------------------------------------------
// Cone material
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ConeKey {
    march: bool,
    ballast: bool,
}

/// Additive cone material (mesh or ray-marched, see the shader).
#[derive(Asset, AsBindGroup, TypePath, Clone)]
#[bind_group_data(ConeKey)]
struct ConeMaterial {
    #[uniform(0)]
    params: Vec4,
    #[uniform(1)]
    params2: Vec4,
    march: bool,
    ballast: bool,
}

impl From<&ConeMaterial> for ConeKey {
    fn from(m: &ConeMaterial) -> Self {
        Self {
            march: m.march,
            ballast: m.ballast,
        }
    }
}

impl Material for ConeMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if key.bind_group_data.ballast {
            descriptor.primitive.cull_mode = None;
            if let Some(f) = descriptor.fragment.as_mut() {
                f.shader_defs.push("BALLAST".into());
            }
        } else if key.bind_group_data.march {
            descriptor.primitive.cull_mode = Some(Face::Front);
            if let Some(f) = descriptor.fragment.as_mut() {
                f.shader_defs.push("CONE_MARCH".into());
            }
        } else {
            descriptor.primitive.cull_mode = None;
        }
        Ok(())
    }
}

/// Unit cone: apex at the origin, base ring (radius 1) at z = -1, axis -Z, plus a base cap.
fn cone_mesh() -> Mesh {
    const SEG: usize = 24;
    let mut pos: Vec<[f32; 3]> = Vec::new();
    let mut nor: Vec<[f32; 3]> = Vec::new();
    let mut uv: Vec<[f32; 2]> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    // Outward side normals of a cone of half-angle atan(1): (cos, sin, 1) normalised.
    for i in 0..SEG {
        let a0 = i as f32 / SEG as f32 * std::f32::consts::TAU;
        let a1 = (i + 1) as f32 / SEG as f32 * std::f32::consts::TAU;
        let am = 0.5 * (a0 + a1);
        let n = |a: f32| Vec3::new(a.cos(), a.sin(), 1.0).normalize().to_array();
        let base = pos.len() as u32;
        pos.push([0.0, 0.0, 0.0]);
        nor.push(n(am));
        uv.push([0.5, 0.0]);
        pos.push([a0.cos(), a0.sin(), -1.0]);
        nor.push(n(a0));
        uv.push([0.0, 1.0]);
        pos.push([a1.cos(), a1.sin(), -1.0]);
        nor.push(n(a1));
        uv.push([1.0, 1.0]);
        // Counter-clockwise seen from outside.
        idx.extend_from_slice(&[base, base + 1, base + 2]);
    }
    // Cap.
    let c = pos.len() as u32;
    pos.push([0.0, 0.0, -1.0]);
    nor.push([0.0, 0.0, -1.0]);
    uv.push([0.5, 0.5]);
    for i in 0..=SEG {
        let a = i as f32 / SEG as f32 * std::f32::consts::TAU;
        pos.push([a.cos(), a.sin(), -1.0]);
        nor.push([0.0, 0.0, -1.0]);
        uv.push([0.5, 0.5]);
    }
    for i in 0..SEG as u32 {
        idx.extend_from_slice(&[c, c + 2 + i, c + 1 + i]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nor)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(Indices::U32(idx))
}

// ---------------------------------------------------------------------------------------------
// Plan
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Cfg {
    label: String,
    kv: BTreeMap<String, String>,
}

impl Cfg {
    fn f(&self, k: &str, d: f32) -> f32 {
        self.kv.get(k).and_then(|v| v.parse().ok()).unwrap_or(d)
    }
    fn u(&self, k: &str, d: u32) -> u32 {
        self.kv.get(k).and_then(|v| v.parse().ok()).unwrap_or(d)
    }
    fn s<'a>(&'a self, k: &str, d: &'a str) -> &'a str {
        self.kv.get(k).map_or(d, String::as_str)
    }
}

fn parse_kv(s: &str) -> BTreeMap<String, String> {
    s.split(',')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

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

fn arg(name: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == name)
        .and_then(|i| a.get(i + 1).cloned())
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

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as f32) / (u32::MAX >> 1) as f32
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next()
    }
}

fn rgba_image(size: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Image {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            data.extend_from_slice(&f(x, y));
        }
    }
    let mut img = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        mipmap_filter: bevy::image::ImageFilterMode::Linear,
        ..default()
    });
    img
}

fn hash(x: u32, y: u32, s: u32) -> f32 {
    let mut h = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))
        .wrapping_add(s.wrapping_mul(2_246_822_519));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0xffff) as f32 / 65535.0
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    win: Res<WindowModeFlag>,
) {
    let mut rng = Rng(0x1234_5678_9abc_def1);
    let cone_handle = meshes.add(cone_mesh());

    // Facade textures: brick-ish with window grid; 6 variants.
    let facade: Vec<Handle<StandardMaterial>> = (0..6)
        .map(|v| {
            let tint = [
                [200u8, 160, 140],
                [190, 190, 175],
                [170, 140, 120],
                [210, 200, 170],
                [160, 165, 175],
                [185, 150, 130],
            ][v as usize];
            let img = rgba_image(512, |x, y| {
                let brick = ((x / 16 + (y / 8) % 2 * 8) % 32 < 1 || y % 8 == 0) as u32;
                let win_cell =
                    (x % 128 > 36 && x % 128 < 92 && y % 128 > 30 && y % 128 < 96) as u32;
                let n = hash(x, y, v) * 0.15 + 0.85;
                let base = if win_cell == 1 {
                    [60.0, 80.0, 105.0]
                } else if brick == 1 {
                    [120.0, 115.0, 110.0]
                } else {
                    [tint[0] as f32, tint[1] as f32, tint[2] as f32]
                };
                [
                    (base[0] * n) as u8,
                    (base[1] * n) as u8,
                    (base[2] * n) as u8,
                    255,
                ]
            });
            materials.add(StandardMaterial {
                base_color_texture: Some(images.add(img)),
                perceptual_roughness: 0.9,
                ..default()
            })
        })
        .collect();

    // Boxes: ~300, a street canyon along -Z (x = +-9..40), a few lower annexes.
    let box_meshes: Vec<Handle<Mesh>> = (0..12)
        .map(|i| {
            let w = 8.0 + (i % 4) as f32 * 2.0;
            let h = 8.0 + (i % 5) as f32 * 4.0;
            let d = 9.0 + (i % 3) as f32 * 2.0;
            meshes.add(Cuboid::new(w, h, d))
        })
        .collect();
    let mut boxes = 0;
    'outer: for bld in 0..24 {
        for side in [-1.0f32, 1.0] {
            for part in 0..7 {
                if boxes >= 300 {
                    break 'outer;
                }
                let mi = (rng.next() * 12.0) as usize % 12;
                let mesh = box_meshes[mi].clone();
                let h = 8.0 + (mi % 5) as f32 * 4.0;
                let w = 8.0 + (mi % 4) as f32 * 2.0;
                let z = -(bld as f32) * 11.0 - 6.0;
                let x = side * (11.0 + (part as f32) * 10.0 * rng.range(0.6, 1.0) + w * 0.5);
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(facade[(bld + part + boxes) % 6].clone()),
                    Transform::from_xyz(x, h * 0.5, z + rng.range(-1.5, 1.5)),
                ));
                boxes += 1;
            }
        }
    }

    // Ground and road.
    let road = rgba_image(256, |x, y| {
        let n = hash(x, y, 77) * 0.2 + 0.8;
        let line = (x / 4) % 64 == 31 && (y / 16) % 2 == 0;
        let v = if line { 220.0 } else { 70.0 * n };
        [v as u8, v as u8, v as u8, 255]
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(120.0, 330.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(images.add(road)),
            uv_transform: bevy::math::Affine2::from_scale(Vec2::new(8.0, 40.0)),
            perceptual_roughness: 0.95,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, -150.0),
        NotShadowCaster,
    ));

    // Trees: 100 alpha-masked quads (two crossing quads would be 200; use one).
    let tree_mats: Vec<Handle<StandardMaterial>> = (0..2)
        .map(|v| {
            let img = rgba_image(256, |x, y| {
                let (fx, fy) = (x as f32 / 255.0 - 0.5, y as f32 / 255.0);
                let r = ((fx * 1.6).powi(2) + ((fy - 0.38) * 1.2).powi(2)).sqrt();
                let leaf = hash(x / 3, y / 3, v + 3) > 0.25 + r;
                let trunk = fx.abs() < 0.03 && fy > 0.6;
                if (r < 0.45 && leaf) || trunk {
                    let g = 80.0 + hash(x, y, v) * 70.0;
                    [(g * 0.5) as u8, g as u8, (g * 0.4) as u8, 255]
                } else {
                    [0, 0, 0, 0]
                }
            });
            materials.add(StandardMaterial {
                base_color_texture: Some(images.add(img)),
                alpha_mode: AlphaMode::Mask(0.5),
                double_sided: true,
                cull_mode: None,
                perceptual_roughness: 0.9,
                ..default()
            })
        })
        .collect();
    let tree_mesh = meshes.add(Rectangle::new(5.0, 7.0));
    for i in 0..100 {
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let z = -4.0 - (i / 2) as f32 * 4.5;
        commands.spawn((
            Mesh3d(tree_mesh.clone()),
            MeshMaterial3d(tree_mats[i % 2].clone()),
            Transform::from_xyz(side * 6.8, 3.5, z).with_rotation(Quat::from_rotation_y(
                rng.range(-0.6, 0.6) + if i % 3 == 0 { 1.2 } else { 0.0 },
            )),
        ));
    }

    // Street lamps: 48 map point lights (as MapLightPool: r = 4.5 m, 10 klx at r).
    let lumens = 4.0 * std::f32::consts::PI * 4.5 * 4.5 * 10_000.0;
    for i in 0..48 {
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        commands.spawn((
            PointLight {
                intensity: lumens,
                range: 36.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(side * 5.5, 6.5, -4.0 - (i / 2) as f32 * 9.0),
            Visibility::Hidden,
            NightLamp(i),
        ));
    }
    // Headlight spots (vehicle_glow.rs: 60 klm, 60 m, 12 / 30 deg).
    for i in 0..8 {
        let lane = if i % 2 == 0 { -2.0 } else { 2.0 };
        let (z, dir) = if i % 2 == 0 {
            (-12.0 - i as f32 * 14.0, Vec3::NEG_Z)
        } else {
            (-45.0 - i as f32 * 10.0, Vec3::Z)
        };
        commands.spawn((
            SpotLight {
                intensity: 60_000.0,
                range: 60.0,
                inner_angle: 12f32.to_radians(),
                outer_angle: 30f32.to_radians(),
                shadow_maps_enabled: false,
                color: Color::srgb(1.0, 0.95, 0.85),
                ..default()
            },
            Transform::from_xyz(lane, 0.8, z).looking_to(dir, Vec3::Y),
            Visibility::Hidden,
            HeadSpot(i),
        ));
    }

    // Sun (SunLight in sky.rs: default DirectionalLight with shadows, default cascades).
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: 20_000.0,
            ..default()
        },
        Transform::default().looking_to(Vec3::NEG_Y, Vec3::Z),
        Sun,
    ));

    // Camera.
    let mut cam = commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.8, 6.0).looking_to(Vec3::NEG_Z, Vec3::Y),
        Tonemapping::default(),
        Msaa::Sample4,
        RenderLayers::from_layers(&[0, 1]),
        Cam,
    ));
    let mut target = None;
    if !win.0 {
        let mut image = Image::new_fill(
            Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_SRC
            | TextureUsages::RENDER_ATTACHMENT;
        let handle = images.add(image);
        cam.insert(RenderTarget::Image(handle.clone().into()));
        target = Some(handle);
    }
    let _ = RenderLayers::layer(0);
    commands.insert_resource(Assets3 {
        cone: cone_handle,
        target,
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
    });
}

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

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn drive(
    mut commands: Commands,
    time: Res<Time<Real>>,
    diag: Res<DiagnosticsStore>,
    mut bench: ResMut<Bench>,
    assets: Res<Assets3>,
    mut cones: ResMut<Assets<ConeMaterial>>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    cam: Query<Entity, With<Cam>>,
    mut images: ResMut<Assets<Image>>,
    mut every: ResMut<MirrorEvery>,
    stale_q: Query<Entity, Or<(With<Cone>, With<Fog>, With<Mirror>)>>,
    mut sun: Query<(Entity, &mut Transform, &mut Visibility), (With<Sun>, Without<Cam>)>,
    mut lamps: Query<(&NightLamp, &mut Visibility), (Without<Sun>, Without<HeadSpot>)>,
    mut spots: Query<
        (Entity, &HeadSpot, &mut SpotLight, &mut Visibility),
        (Without<Sun>, Without<NightLamp>),
    >,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut exit: MessageWriter<AppExit>,
) {
    if bench.idx >= bench.plan.len() {
        exit.write(AppExit::Success);
        return;
    }
    let cfg = bench.plan[bench.idx].clone();
    let warm = cfg.u("warmup", 120);
    let frames = cfg.u("frames", 600);
    let shot_at = warm + 4;
    let measure_from = if cfg.u("shot", 0) == 1 {
        warm + 24
    } else {
        warm
    };

    if bench.frame == 0 {
        apply(
            &cfg,
            &mut commands,
            &assets,
            &mut cones,
            &mut shadow_map,
            cam.single().ok(),
            &mut images,
            &mut every,
            &stale_q,
            &mut sun,
            &mut lamps,
            &mut spots,
            &mut ambient,
        );
        bench.ms.clear();
        bench.gpu.clear();
        bench.item_start = std::time::Instant::now();
        bench.shot_done = false;
        bench.frame = 1;
        return;
    }
    bench.frame += 1;
    let f = bench.frame;
    let warm_s = cfg.f("warm_s", if bench.idx == 0 { 4.0 } else { 1.2 });
    let warmed = bench.item_start.elapsed().as_secs_f32() >= warm_s;
    if f >= shot_at && warmed && !bench.shot_done && cfg.u("shot", 0) == 1 {
        bench.shot_done = true;
        if let Some(dir) = bench.shots.clone() {
            let _ = std::fs::create_dir_all(&dir);
            let shot = match &assets.target {
                Some(h) => Screenshot::image(h.clone()),
                None => Screenshot::primary_window(),
            };
            commands
                .spawn(shot)
                .observe(save_to_disk(dir.join(format!("{}.png", cfg.label))));
        }
    }
    let shot_wait = cfg.u("shot", 0) == 1 && (!bench.shot_done || f < shot_at + 20);
    if f > measure_from && warmed && !shot_wait {
        bench.ms.push(time.delta_secs() * 1000.0);
        for d in diag.iter() {
            let p = d.path().as_str();
            if p.ends_with("elapsed_gpu") {
                if let Some(v) = d.value() {
                    bench.gpu.entry(p.to_string()).or_default().push(v);
                }
            }
        }
    }
    let over_time = bench.started.elapsed().as_secs_f32() > bench.max_seconds;
    if bench.ms.len() as u32 >= frames || over_time {
        let mut ms = bench.ms.clone();
        ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = ms.len().max(1) as f32;
        let mean = ms.iter().sum::<f32>() / n;
        let mut gpu = serde_json::Map::new();
        for (k, v) in &bench.gpu {
            let mut s = v.clone();
            s.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let med = s[s.len() / 2];
            gpu.insert(
                k.clone(),
                serde_json::json!((med * 1000.0).round() / 1000.0),
            );
        }
        let gpu_sum: f64 = bench
            .gpu
            .iter()
            .filter(|(k, _)| {
                k.contains("main_opaque_pass_3d")
                    || k.contains("main_transparent_pass_3d")
                    || k.contains("main_transmissive")
                    || k.contains("shadow")
                    || k.contains("volumetric")
                    || k.contains("prepass")
                    || k.contains("post_processing")
                    || k.contains("tonemapping")
                    || k.contains("msaa")
            })
            .map(|(_, v)| {
                let mut s = v.clone();
                s.sort_by(|a, b| a.partial_cmp(b).unwrap());
                s[s.len() / 2]
            })
            .sum();
        let rec = serde_json::json!({
            "label": cfg.label,
            "cfg": cfg.kv,
            "frames": ms.len(),
            "median_ms": pct(&ms, 0.5),
            "p95_ms": pct(&ms, 0.95),
            "p99_ms": pct(&ms, 0.99),
            "max_ms": ms.last().copied().unwrap_or(0.0),
            "min_fps": 1000.0 / ms.last().copied().unwrap_or(1.0).max(0.001),
            "mean_ms": mean,
            "gpu_median_ms": gpu,
            "gpu_sum_known_ms": gpu_sum,
            "loadavg_start": bench.load,
            "truncated": over_time,
        });
        let line = rec.to_string();
        println!("RESULT {line}");
        if let Some(out) = &bench.out {
            if let Ok(mut fh) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(out)
            {
                let _ = writeln!(fh, "{line}");
            }
        }
        bench.idx += 1;
        bench.frame = 0;
        if over_time {
            bench.idx = bench.plan.len();
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn apply(
    cfg: &Cfg,
    commands: &mut Commands,
    assets: &Assets3,
    cones: &mut Assets<ConeMaterial>,
    shadow_map: &mut DirectionalLightShadowMap,
    cam: Option<Entity>,
    images: &mut Assets<Image>,
    every: &mut MirrorEvery,
    stale_q: &Query<Entity, Or<(With<Cone>, With<Fog>, With<Mirror>)>>,
    sun: &mut Query<(Entity, &mut Transform, &mut Visibility), (With<Sun>, Without<Cam>)>,
    lamps: &mut Query<(&NightLamp, &mut Visibility), (Without<Sun>, Without<HeadSpot>)>,
    spots: &mut Query<
        (Entity, &HeadSpot, &mut SpotLight, &mut Visibility),
        (Without<Sun>, Without<NightLamp>),
    >,
    ambient: &mut GlobalAmbientLight,
) {
    let Some(cam) = cam else { return };
    let night = cfg.s("scene", "sun") == "night";
    shadow_map.size = cfg.u("shadow", 2048) as usize;
    commands.insert_resource(ClearColor(if night {
        Color::srgb(0.01, 0.015, 0.03)
    } else {
        Color::srgb(0.52, 0.70, 0.95)
    }));

    // Sun.
    let elev = cfg.f("elev", 10.0).to_radians();
    let toward = cfg.s("look", "toward") == "toward";
    let moon = cfg.u("moon", 0) == 1;
    let az = 35f32.to_radians();
    let sz = if toward { -az.cos() } else { az.cos() };
    let sun_dir = Vec3::new(az.sin() * elev.cos(), elev.sin(), sz * elev.cos()).normalize();
    for (e, mut t, mut vis) in sun {
        let casc = CascadeShadowConfigBuilder {
            num_cascades: cfg.u("casc", 4) as usize,
            maximum_distance: cfg.f("cmax", 150.0),
            ..default()
        }
        .build();
        commands.entity(e).insert(casc);
        if let Ok(mut l) = commands.get_entity(e) {
            l.insert(sun_light(cfg, moon));
        }
        if cfg.u("fog", 0) == 1 && (!night || moon) {
            commands.entity(e).insert(VolumetricLight);
        } else {
            commands.entity(e).remove::<VolumetricLight>();
        }
        *t = Transform::default().looking_to(-sun_dir, Vec3::Y);
        *vis = if night && !moon {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    ambient.brightness = if night { 20.0 } else { 700.0 };
    let lamp_n = cfg.u("lamps", 48) as usize;
    for (l, mut v) in lamps {
        *v = if night && l.0 < lamp_n {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let spot_shadows = cfg.u("spotsh", 0) as usize;
    let vspots = cfg.u("vspots", 0) as usize;
    for (e, h, mut s, mut v) in spots {
        *v = if night && h.0 < cfg.u("spots", 6).max(vspots as u32) as usize {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        s.shadow_maps_enabled = h.0 < spot_shadows.max(vspots);
        if h.0 < vspots {
            commands.entity(e).insert(VolumetricLight);
        } else {
            commands.entity(e).remove::<VolumetricLight>();
        }
    }

    // Camera.
    let az_cam = if toward { 0.0 } else { std::f32::consts::PI };
    // "toward" looks down -Z (toward the sun); "away" turns around and looks down +Z, the camera
    // sits at the far end of the street so the same canyon fills the view.
    let pos = if toward {
        Vec3::new(0.0, 1.8, 6.0)
    } else {
        Vec3::new(0.0, 1.8, -140.0)
    };
    let _ = az_cam;
    let look_dir = if toward { Vec3::NEG_Z } else { Vec3::Z };
    commands
        .entity(cam)
        .insert(Transform::from_translation(pos).looking_to(look_dir, Vec3::Y));
    if cfg.u("msaa", 4) == 0 {
        commands.entity(cam).insert(Msaa::Off);
    } else if cfg.u("msaa", 4) == 2 {
        commands.entity(cam).insert(Msaa::Sample2);
    } else {
        commands.entity(cam).insert(Msaa::Sample4);
    }
    if cfg.u("hdr", 0) == 1 {
        commands.entity(cam).insert(Hdr);
    } else {
        commands.entity(cam).remove::<Hdr>();
    }
    if cfg.u("bloom", 0) == 1 {
        let mut b = Bloom::NATURAL;
        b.intensity = cfg.f("bloomi", 0.05);
        commands.entity(cam).insert(b);
    } else {
        commands.entity(cam).remove::<Bloom>();
    }
    if cfg.u("ibl", 0) == 1 {
        commands.entity(cam).insert(GeneratedEnvironmentMapLight {
            environment_map: images.add(sky_cube(64)),
            intensity: 500.0,
            ..default()
        });
    } else {
        commands
            .entity(cam)
            .remove::<GeneratedEnvironmentMapLight>();
    }
    // Mirror cameras (render to a small 8-bit image; every `mevery`-th frame).
    for e in stale_q {
        commands.entity(e).despawn();
    }
    every.0 = cfg.u("mevery", 1);
    let mw = cfg.u("msize", 256);
    for i in 0..cfg.u("mirrors", 0) as usize {
        let mut image = Image::new_fill(
            Extent3d {
                width: mw,
                height: mw / 2,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_SRC
            | TextureUsages::RENDER_ATTACHMENT;
        let handle = images.add(image);
        let mut m = commands.spawn((
            Camera3d::default(),
            Camera {
                order: -1 - i as isize,
                ..default()
            },
            RenderTarget::Image(handle.into()),
            Msaa::Off,
            Transform::from_translation(pos + Vec3::new(if i == 0 { 2.5 } else { -2.5 }, 0.3, 1.0))
                .looking_to(look_dir, Vec3::Y),
            Mirror,
        ));
        if cfg.u("mhdr", 0) == 1 {
            m.insert(Hdr);
        }
        if cfg.u("mibl", 0) == 1 {
            m.insert(GeneratedEnvironmentMapLight {
                environment_map: images.add(sky_cube(64)),
                intensity: 500.0,
                ..default()
            });
        }
    }
    let ballast = cfg.u("ballast", 0);
    if ballast > 0 {
        let mat = cones.add(ConeMaterial {
            params: Vec4::ZERO,
            params2: Vec4::new(ballast as f32, 0.0, 0.0, 0.0),
            march: false,
            ballast: true,
        });
        commands.spawn((
            Mesh3d(assets.quad.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(pos + look_dir * 1.0)
                .looking_to(look_dir, Vec3::Y)
                .with_scale(Vec3::new(1.7, 1.0, 1.0)),
            NotShadowCaster,
            NotShadowReceiver,
            RenderLayers::layer(1),
            bevy::camera::visibility::NoFrustumCulling,
            Cone,
        ));
    }
    let tech = cfg.s("tech", "none");
    let need_prepass = cfg.u("prepass", 0) == 1 || cfg.u("fade", 0) == 1;
    if need_prepass {
        commands.entity(cam).insert(DepthPrepass);
    } else {
        commands.entity(cam).remove::<DepthPrepass>();
    }
    if cfg.u("fog", 0) == 1 || vspots > 0 {
        commands.entity(cam).insert(VolumetricFog {
            step_count: cfg.u("steps", 64),
            jitter: cfg.f("jitter", 0.0),
            ambient_intensity: cfg.f("ambient", 0.0),
            ambient_color: Color::WHITE,
        });
    } else {
        commands.entity(cam).remove::<VolumetricFog>();
    }

    // Fog volume.
    if cfg.u("fog", 0) == 1 || vspots > 0 {
        let side = cfg.f("vol", 200.0);
        commands.spawn((
            FogVolume {
                density_factor: cfg.f("density", 0.002),
                scattering: 0.3,
                absorption: 0.1,
                scattering_asymmetry: cfg.f("g", 0.6),
                ..default()
            },
            Transform::from_translation(pos).with_scale(Vec3::splat(side)),
            Fog,
        ));
    }

    // Cones.
    let n = cfg.u("n", 0) as usize;
    if n > 0 && (tech == "mesh" || tech == "march") {
        let march = tech == "march";
        let fade = cfg.u("fade", 0) == 1;
        let mat_l = cones.add(ConeMaterial {
            params: Vec4::new(
                if march {
                    cfg.f("inten", 3.0)
                } else {
                    cfg.f("inten", 0.45)
                },
                cfg.f("edge", 2.0),
                if fade { 1.5 } else { 0.0 },
                cfg.f("axial", 1.5),
            ),
            params2: Vec4::new(cfg.u("msteps", 8) as f32, 0.35, 0.0, cfg.f("dbg", 0.0)),
            march,
            ballast: false,
        });
        if cfg.u("inside", 0) == 1 {
            // A big cone around the camera: the view starts inside its volume.
            let (len, half) = (40.0f32, 22f32.to_radians());
            commands.spawn((
                Mesh3d(assets.cone.clone()),
                MeshMaterial3d(mat_l.clone()),
                Transform::from_translation(pos - look_dir * 3.0 + Vec3::new(0.0, 0.3, 0.0))
                    .looking_to(look_dir, Vec3::Y)
                    .with_scale(Vec3::new(len * half.tan(), len * half.tan(), len)),
                MeshTag::new(255 | (240 << 8) | (200 << 16)),
                NotShadowCaster,
                NotShadowReceiver,
                Cone,
            ));
        }
        let mut rng = Rng(99);
        for i in 0..n {
            let lantern = i % 5 < 3;
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            let zf = (i as f32 + 0.5) / n as f32;
            let z = -8.0 - zf * 120.0;
            let (apex, dir, len, half) = if lantern {
                (
                    Vec3::new(side * 5.5, 6.5, z),
                    Vec3::new(-side * 0.25, -1.0, 0.0).normalize(),
                    8.0,
                    30f32.to_radians(),
                )
            } else {
                let lane = if rng.next() < 0.5 { -2.0 } else { 2.0 };
                (
                    Vec3::new(lane + rng.range(-0.5, 0.5), 0.8, z),
                    if lane < 0.0 { Vec3::NEG_Z } else { Vec3::Z },
                    35.0,
                    14f32.to_radians(),
                )
            };
            let col = if lantern {
                [255u32, 215, 150]
            } else {
                [255, 245, 215]
            };
            let tag = col[0] | (col[1] << 8) | (col[2] << 16);
            commands.spawn((
                Mesh3d(assets.cone.clone()),
                MeshMaterial3d(mat_l.clone()),
                Transform::from_translation(apex)
                    .looking_to(dir, Vec3::Y)
                    .with_scale(Vec3::new(len * half.tan(), len * half.tan(), len)),
                MeshTag::new(tag),
                NotShadowCaster,
                NotShadowReceiver,
                Cone,
            ));
        }
    }
}

fn sun_light(cfg: &Cfg, moon: bool) -> DirectionalLight {
    DirectionalLight {
        shadow_maps_enabled: cfg.u("sunshadow", 1) == 1,
        illuminance: if moon {
            cfg.f("lux", 400.0)
        } else {
            cfg.f("lux", 20_000.0)
        },
        ..default()
    }
}

fn mirror_toggle(
    bench: Res<Bench>,
    every: Res<MirrorEvery>,
    mut q: Query<&mut Camera, With<Mirror>>,
) {
    let on = every.0 <= 1 || bench.frame.is_multiple_of(every.0);
    for mut c in &mut q {
        if c.is_active != on {
            c.is_active = on;
        }
    }
}

fn f16_bits(v: f32) -> u16 {
    let v = v.clamp(0.0, 65_000.0);
    if v < 6.1e-5 {
        return 0;
    }
    let bits = v.to_bits();
    let exp = ((bits >> 23) as i32) - 127 + 15;
    let man = (bits >> 13) & 0x3ff;
    let round = (bits >> 12) & 1;
    ((((exp as u32) << 10) | man) + round) as u16
}

/// Simple sky-gradient cubemap (Rgba16Float) for the IBL cost test.
fn sky_cube(size: u32) -> Image {
    let mut data = Vec::new();
    for face in 0..6 {
        for y in 0..size {
            for x in 0..size {
                let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let v = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let d = match face {
                    0 => Vec3::new(1.0, -v, -u),
                    1 => Vec3::new(-1.0, -v, u),
                    2 => Vec3::new(u, 1.0, v),
                    3 => Vec3::new(u, -1.0, -v),
                    4 => Vec3::new(u, -v, 1.0),
                    _ => Vec3::new(-u, -v, -1.0),
                }
                .normalize();
                let c = if d.y >= 0.0 {
                    Vec3::new(0.8, 0.88, 1.0).lerp(Vec3::new(0.18, 0.36, 0.85), d.y.sqrt())
                } else {
                    Vec3::new(0.16, 0.15, 0.14)
                };
                for ch in [c.x, c.y, c.z, 1.0] {
                    data.extend_from_slice(&f16_bits(ch).to_le_bytes());
                }
            }
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba16Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_view_descriptor = Some(bevy::render::render_resource::TextureViewDescriptor {
        dimension: Some(bevy::render::render_resource::TextureViewDimension::Cube),
        ..default()
    });
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    image
}
