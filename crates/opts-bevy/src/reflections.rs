//! Research prototypes for reflections (M-0085): planar rear-view mirrors, glass with
//! environment reflection or transmission, and a wet-surface parameter set driven by [`Wetness`].
//!
//! Everything here is **off by default**. The switches come from the environment variable
//! `OPTS_REFLECT` (comma separated), read once at start-up:
//!
//! | Switch | Effect |
//! | :--- | :--- |
//! | `env` | procedural sky cubemap as image-based light on the main camera (`GeneratedEnvironmentMapLight`) |
//! | `glass=blend` | glass materials: low roughness, `AlphaMode::Blend` (needs `env` to reflect anything) |
//! | `glass=premul` | the same with `AlphaMode::Premultiplied` (reflection survives high transparency) |
//! | `glass=trans` | `specular_transmission` + `ScreenSpaceTransmission` on the camera |
//! | `mirror` | one extra camera per mirror mesh renders into a texture shown on the mirror |
//! | `mirror_size=256x128`, `mirror_every=2`, `mirror_max=2` | resolution, render every Nth frame, mirror count |
//! | `wet` | [`Wetness`]-driven roughness drop, darkening and puddles on ground and standard materials |
//! | `ssr` | screen-space reflections (switches the camera to deferred rendering, MSAA off) |
//!
//! `OPTS_WETNESS=0..1` sets the initial [`Wetness`]. `OPTS_REFLECT_STATS=<file>` makes the plugin
//! write frame-time statistics (after `OPTS_REFLECT_STATS_AFTER` seconds, default 20) and exit.
//!
//! Prototype quality: see `docs/src/architecture/reflections-research.md` for what it proves
//! and what it does not.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

use bevy::asset::RenderAssetUsages;
use bevy::camera::RenderTarget;
use bevy::light::GeneratedEnvironmentMapLight;
use bevy::pbr::{ScreenSpaceReflections, ScreenSpaceTransmission, StandardMaterial};
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
    TextureViewDimension,
};

use crate::ground_material::GroundMaterial;

/// How glass materials are treated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlassMode {
    /// Untouched (default).
    Off,
    /// Low roughness, alpha blended.
    Blend,
    /// Low roughness, premultiplied alpha (keeps specular on transparent glass).
    Premul,
    /// Screen-space specular transmission.
    Transmission,
}

/// The parsed `OPTS_REFLECT` switches.
#[derive(Debug, Clone, Resource)]
pub struct ReflectionSwitches {
    /// Environment cubemap on the main camera.
    pub env: bool,
    /// Glass treatment.
    pub glass: GlassMode,
    /// Planar mirrors.
    pub mirror: bool,
    /// Mirror texture size.
    pub mirror_size: UVec2,
    /// Render the mirror camera every N-th frame.
    pub mirror_every: u32,
    /// Most mirrors that get a camera.
    pub mirror_max: usize,
    /// Wet-surface parameter set.
    pub wet: bool,
    /// Screen-space reflections (deferred).
    pub ssr: bool,
    /// Show the first mirror texture on a quad in front of the main camera (debug aid).
    pub mirror_debug: bool,
    /// Environment intensity (cd/m2).
    pub env_intensity: f32,
}

impl ReflectionSwitches {
    /// Parses a switch list such as `env,glass=trans,mirror,mirror_size=256x128`.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut s = Self {
            env: false,
            glass: GlassMode::Off,
            mirror: false,
            mirror_size: UVec2::new(256, 128),
            mirror_every: 1,
            mirror_max: 2,
            wet: false,
            ssr: false,
            mirror_debug: false,
            env_intensity: 3000.0,
        };
        for item in text.split(',').map(str::trim).filter(|i| !i.is_empty()) {
            let (key, value) = item.split_once('=').unwrap_or((item, ""));
            match key {
                "env" => s.env = true,
                "glass" => {
                    s.glass = match value {
                        "premul" => GlassMode::Premul,
                        "trans" => GlassMode::Transmission,
                        _ => GlassMode::Blend,
                    }
                }
                "mirror" => s.mirror = true,
                "mirror_size" => {
                    if let Some((w, h)) = value.split_once('x') {
                        if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) {
                            s.mirror_size = UVec2::new(w, h);
                        }
                    }
                }
                "mirror_every" => s.mirror_every = value.parse().unwrap_or(1).max(1),
                "mirror_max" => s.mirror_max = value.parse().unwrap_or(2),
                "wet" => s.wet = true,
                "ssr" => s.ssr = true,
                "mirror_debug" => s.mirror_debug = true,
                "env_intensity" => s.env_intensity = value.parse().unwrap_or(3000.0),
                _ => {}
            }
        }
        s
    }

    /// Reads `OPTS_REFLECT`; all off when unset.
    #[must_use]
    pub fn from_env() -> Self {
        Self::parse(&std::env::var("OPTS_REFLECT").unwrap_or_default())
    }

    /// Whether any switch is on.
    #[must_use]
    pub fn any(&self) -> bool {
        self.env
            || self.glass != GlassMode::Off
            || self.mirror
            || self.wet
            || self.ssr
    }
}

/// How wet the world is, 0 (dry) to 1 (soaked). The single driver of the wet-surface parameters;
/// a weather system would write it.
#[derive(Debug, Clone, Copy, Default, Resource)]
pub struct Wetness(pub f32);

/// What the model loader recorded about one material, enough to treat it afterwards.
#[derive(Debug, Clone)]
pub struct MaterialNote {
    /// Texture file name of the material, lower case.
    pub texture: String,
    /// Original alpha mode.
    pub alpha: AlphaMode,
    /// Original base colour.
    pub base_color: Color,
    /// Original perceptual roughness.
    pub roughness: f32,
}

static SWITCHES: OnceLock<ReflectionSwitches> = OnceLock::new();
static NOTES: Mutex<Option<HashMap<AssetId<StandardMaterial>, MaterialNote>>> = Mutex::new(None);

/// The switches read from the environment once.
pub fn switches() -> &'static ReflectionSwitches {
    SWITCHES.get_or_init(ReflectionSwitches::from_env)
}

/// Called by the model loader for every material it creates; free when no switch is on.
pub fn note_material(handle: &Handle<StandardMaterial>, texture: Option<&str>, m: &StandardMaterial) {
    if !switches().any() {
        return;
    }
    let note = MaterialNote {
        texture: texture.unwrap_or("").to_lowercase(),
        alpha: m.alpha_mode,
        base_color: m.base_color,
        roughness: m.perceptual_roughness,
    };
    if let Ok(mut g) = NOTES.lock() {
        g.get_or_insert_with(HashMap::new).insert(handle.id(), note);
    }
}

fn note_of(id: AssetId<StandardMaterial>) -> Option<MaterialNote> {
    NOTES.lock().ok()?.as_ref()?.get(&id).cloned()
}

fn texture_stem(note: &MaterialNote) -> &str {
    note.texture
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&note.texture)
}

/// Glass by texture name (the corpus has no other marker; see the research report).
#[must_use]
pub fn is_glass_name(name: &str) -> bool {
    ["glas", "glass", "scheibe", "fenster", "window", "verglas", "windscreen", "windshield"]
        .iter()
        .any(|w| name.contains(w))
}

/// Mirror by texture name.
#[must_use]
pub fn is_mirror_name(name: &str) -> bool {
    ["spiegel", "mirror"].iter().any(|w| name.contains(w))
}

/// Marks the main camera once it carries the prototype components.
#[derive(Component)]
struct ReflectionsApplied;

/// Marks a mirror camera.
#[derive(Component)]
struct MirrorCamera {
    mirror: Entity,
    /// Mirror surface centre and mean normal in the mirror mesh's local space.
    centre: Vec3,
    normal: Vec3,
}

/// Marks a mirror mesh that has its camera.
#[derive(Component)]
struct MirrorDone;

/// Registers the prototypes; does nothing unless a switch is on.
pub struct ReflectionsResearchPlugin;

impl Plugin for ReflectionsResearchPlugin {
    fn build(&self, app: &mut App) {
        let sw = switches().clone();
        if !sw.any() && std::env::var_os("OPTS_REFLECT_STATS").is_none() {
            return;
        }
        let wet0 = std::env::var("OPTS_WETNESS")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(0.0);
        app.insert_resource(sw.clone())
            .insert_resource(Wetness(wet0.clamp(0.0, 1.0)))
            .init_resource::<GlassDone>()
            .init_resource::<WetState>()
            .init_resource::<FrameStats>();
        if std::env::var_os("OPTS_REFLECT_GPU").is_some() {
            app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
        }
        app.add_systems(Update, (apply_camera_features, apply_glass, apply_wet, collect_stats));
        if sw.mirror {
            app.add_systems(Update, spawn_mirror_cameras)
                .add_systems(
                    PostUpdate,
                    update_mirror_cameras.before(bevy::transform::TransformSystems::Propagate),
                )
                // The camera systems of the game query `With<Camera3d>` and assume exactly one;
                // the mirror cameras are `Camera3d` only while the frame is extracted.
                .add_systems(First, hide_mirror_cameras)
                .add_systems(Last, show_mirror_cameras);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Environment cubemap, transmission, SSR
// ---------------------------------------------------------------------------------------------

fn f16_bits(v: f32) -> u16 {
    // Positive finite values only (radiance), round to nearest, flush tiny to zero, clamp huge.
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

fn sky_radiance(d: Vec3) -> Vec3 {
    let up = d.y;
    let zenith = Vec3::new(0.18, 0.36, 0.85);
    let horizon = Vec3::new(0.80, 0.88, 1.00);
    let ground = Vec3::new(0.16, 0.15, 0.14);
    let mut c = if up >= 0.0 {
        horizon.lerp(zenith, up.powf(0.5))
    } else {
        horizon.lerp(ground, (-up * 6.0).min(1.0))
    };
    let sun = Vec3::new(0.45, 0.6, 0.35).normalize();
    c += Vec3::splat(40.0) * d.dot(sun).max(0.0).powf(400.0);
    c
}

/// A small procedural sky cubemap (a stand-in for a cubemap captured from the sky dome).
#[must_use]
pub fn procedural_sky_cube(size: u32) -> Image {
    let mut data = Vec::with_capacity((size * size * 6 * 8) as usize);
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
                let c = sky_radiance(d);
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
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    image
}

fn apply_camera_features(
    sw: Res<ReflectionSwitches>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    cameras: Query<Entity, (With<Camera3d>, Without<ReflectionsApplied>)>,
) {
    for e in &cameras {
        let mut c = commands.entity(e);
        c.insert(ReflectionsApplied);
        if sw.env {
            let cube = images.add(procedural_sky_cube(64));
            c.insert(GeneratedEnvironmentMapLight {
                environment_map: cube,
                intensity: sw.env_intensity,
                ..default()
            });
        }
        if sw.glass == GlassMode::Transmission {
            c.insert(ScreenSpaceTransmission::default());
        }
        if sw.ssr {
            c.insert((
                Msaa::Off,
                ScreenSpaceReflections::default(),
                bevy::core_pipeline::prepass::DepthPrepass,
                bevy::core_pipeline::prepass::DeferredPrepass,
            ));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Glass
// ---------------------------------------------------------------------------------------------

#[derive(Resource, Default)]
struct GlassDone(HashSet<AssetId<StandardMaterial>>);

fn apply_glass(
    sw: Res<ReflectionSwitches>,
    mut done: ResMut<GlassDone>,
    mut glass_count: Local<(u32,)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if sw.glass == GlassMode::Off {
        return;
    }
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        if done.0.contains(&id) {
            continue;
        }
        let Some(note) = note_of(id) else { continue };
        done.0.insert(id);
        if !is_glass_name(texture_stem(&note)) || matches!(note.alpha, AlphaMode::Opaque) {
            continue;
        }
        let Some(mut m) = materials.get_mut(id) else { continue };
        glass_count.0 += 1;
        if glass_count.0 % 50 == 1 {
            bevy::log::info!(count = glass_count.0, texture = %note.texture, "reflections: glass material treated");
        }
        m.perceptual_roughness = 0.06;
        m.reflectance = 0.6;
        match sw.glass {
            GlassMode::Blend => m.alpha_mode = AlphaMode::Blend,
            GlassMode::Premul => m.alpha_mode = AlphaMode::Premultiplied,
            GlassMode::Transmission => {
                m.specular_transmission = 0.9;
                m.thickness = 0.02;
                m.ior = 1.5;
                m.alpha_mode = AlphaMode::Opaque;
                m.base_color = Color::srgb(0.85, 0.92, 0.9);
                m.base_color_texture = None;
            }
            GlassMode::Off => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Wet surfaces
// ---------------------------------------------------------------------------------------------

#[derive(Resource, Default)]
struct WetState {
    last: f32,
    applied_count: usize,
}

/// Roughness and colour of a standard material at wetness `w`.
#[must_use]
pub fn wet_standard(base: Color, roughness: f32, w: f32) -> (Color, f32) {
    let c = base.to_linear();
    let k = 1.0 - 0.35 * w;
    let wet = Color::linear_rgba(c.red * k, c.green * k, c.blue * k, c.alpha);
    (wet, roughness * (1.0 - 0.65 * w))
}

fn apply_wet(
    sw: Res<ReflectionSwitches>,
    wetness: Res<Wetness>,
    mut state: ResMut<WetState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ground: ResMut<Assets<GroundMaterial>>,
) {
    if !sw.wet {
        return;
    }
    let w = wetness.0;
    let ground_count = ground.len();
    if (w - state.last).abs() < 1e-4 && state.applied_count == materials.len() + ground_count {
        return;
    }
    state.last = w;
    state.applied_count = materials.len() + ground_count;
    let ids: Vec<_> = materials.ids().collect();
    for id in ids {
        let Some(note) = note_of(id) else { continue };
        if !matches!(note.alpha, AlphaMode::Opaque) {
            continue;
        }
        if let Some(mut m) = materials.get_mut(id) {
            let (c, r) = wet_standard(note.base_color, note.roughness, w);
            m.base_color = c;
            m.perceptual_roughness = r;
        }
    }
    let gids: Vec<_> = ground.ids().collect();
    for id in gids {
        if let Some(mut m) = ground.get_mut(id) {
            m.extension.wet = Vec4::new(w, 0.45, 0.0, 0.0);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Planar mirrors
// ---------------------------------------------------------------------------------------------

fn spawn_mirror_cameras(
    sw: Res<ReflectionSwitches>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    existing: Query<(), With<MirrorCamera>>,
    main_cams: Query<Entity, (With<Camera3d>, Without<MirrorCamera>)>,
    mut meshes_add: ResMut<Assets<Mesh>>,
    candidates: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>, &Mesh3d, &GlobalTransform),
        Without<MirrorDone>,
    >,
) {
    let mut count = existing.iter().count();
    for (entity, material, mesh, gt) in &candidates {
        if count >= sw.mirror_max {
            return;
        }
        let Some(note) = note_of(material.0.id()) else { continue };
        if !is_mirror_name(texture_stem(&note)) {
            continue;
        }
        if gt.affine() == bevy::math::Affine3A::IDENTITY {
            continue;
        }
        let Some(mesh) = meshes_add.get(&mesh.0) else { continue };
        let (Some(pos), Some(nor)) = (
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| a.as_float3()),
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).and_then(|a| a.as_float3()),
        ) else {
            continue;
        };
        if pos.is_empty() {
            continue;
        }
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        let mut nsum = Vec3::ZERO;
        for p in pos {
            lo = lo.min(Vec3::from(*p));
            hi = hi.max(Vec3::from(*p));
        }
        for n in nor {
            nsum += Vec3::from(*n);
        }
        let centre = (lo + hi) * 0.5;
        let normal = nsum.normalize_or_zero();
        if normal == Vec3::ZERO {
            continue;
        }
        let size = sw.mirror_size;
        let mut image = Image::new_target_texture(size.x, size.y, TextureFormat::Bgra8UnormSrgb, None);
        image.sampler = bevy::image::ImageSampler::linear();
        let handle = images.add(image);
        // The mirror shows the render unlit, mirrored horizontally.
        if let Some(mut m) = materials.get_mut(material.0.id()) {
            m.base_color_texture = Some(handle.clone());
            m.base_color = Color::WHITE;
            m.unlit = true;
            m.alpha_mode = AlphaMode::Opaque;
            m.double_sided = true;
            m.cull_mode = None;
            m.uv_transform = bevy::math::Affine2::from_cols(
                Vec2::new(-1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(1.0, 0.0),
            );
        }
        bevy::log::info!(texture = %note.texture, ?size, ?normal, centre = ?gt.transform_point(centre), "reflections: mirror camera created");
        if sw.mirror_debug && count == 0 {
            // A picture-in-picture of the first mirror, to judge the render without finding the mirror.
            let quad = meshes_add.add(Rectangle::new(0.6, 0.3));
            let mat = materials.add(StandardMaterial {
                base_color_texture: Some(handle.clone()),
                unlit: true,
                ..default()
            });
            for cam in &main_cams {
                commands.entity(cam).with_child((
                    Mesh3d(quad.clone()),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(0.0, -0.22, -0.9),
                ));
            }
        }
        commands.entity(entity).insert(MirrorDone);
        commands.spawn((
            Camera3d::default(),
            Camera {
                order: -10,
                clear_color: ClearColorConfig::Custom(Color::srgb(0.5, 0.6, 0.8)),
                ..default()
            },
            RenderTarget::Image(handle.into()),
            Msaa::Off,
            Projection::Perspective(PerspectiveProjection {
                fov: 40f32.to_radians(),
                near: 0.2,
                aspect_ratio: size.x as f32 / size.y as f32,
                ..default()
            }),
            Transform::from_translation(gt.transform_point(centre)),
            MirrorCamera {
                mirror: entity,
                centre,
                normal,
            },
            Name::new("Mirror_Camera"),
        ));
        count += 1;
    }
}

fn update_mirror_cameras(
    sw: Res<ReflectionSwitches>,
    mut frame: Local<u32>,
    main: Query<&Transform, (With<Camera3d>, Without<MirrorCamera>)>,
    mirrors: Query<&GlobalTransform, Without<MirrorCamera>>,
    mut cams: Query<(&MirrorCamera, &mut Transform, &mut Camera)>,
) {
    *frame = frame.wrapping_add(1);
    let Some(viewer) = main.iter().next().map(|t| t.translation) else {
        return;
    };
    for (info, mut t, mut cam) in &mut cams {
        let Ok(gt) = mirrors.get(info.mirror) else { continue };
        // Mirror pose of the current frame (the vehicle moves).
        let c = gt.transform_point(info.centre);
        let mut n = gt.affine().matrix3.mul_vec3(info.normal).normalize_or_zero();
        let d = (c - viewer).normalize_or_zero();
        if d.dot(n) > 0.0 {
            n = -n;
        }
        let r = d - 2.0 * d.dot(n) * n;
        t.translation = c + n * 0.05;
        t.look_to(r, Vec3::Y);
        cam.is_active = *frame % sw.mirror_every == 0;
    }
}

fn hide_mirror_cameras(mut commands: Commands, cams: Query<Entity, (With<MirrorCamera>, With<Camera3d>)>) {
    for e in &cams {
        commands.entity(e).remove::<Camera3d>();
    }
}

fn show_mirror_cameras(mut commands: Commands, cams: Query<Entity, (With<MirrorCamera>, Without<Camera3d>)>) {
    for e in &cams {
        commands.entity(e).insert(Camera3d::default());
    }
}

// ---------------------------------------------------------------------------------------------
// Frame statistics
// ---------------------------------------------------------------------------------------------

#[derive(Resource, Default)]
struct FrameStats {
    samples: Vec<f32>,
    started: Option<f64>,
}

fn rss_mib() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        })
        .map_or(0.0, |kb| kb / 1024.0)
}

fn collect_stats(
    time: Res<Time<Real>>,
    mut stats: ResMut<FrameStats>,
    diagnostics: Option<Res<bevy::diagnostic::DiagnosticsStore>>,
    mut exit: MessageWriter<AppExit>,
    cameras: Query<(), With<Camera3d>>,
) {
    let Ok(path) = std::env::var("OPTS_REFLECT_STATS") else { return };
    // Statistics start when the 3D scene is on screen (a camera exists and frames are short).
    if cameras.is_empty() {
        return;
    }
    let now = time.elapsed_secs_f64();
    let start = *stats.started.get_or_insert(now);
    let warmup: f64 = std::env::var("OPTS_REFLECT_WARMUP").ok().and_then(|v| v.parse().ok()).unwrap_or(8.0);
    let span: f64 = std::env::var("OPTS_REFLECT_STATS_AFTER").ok().and_then(|v| v.parse().ok()).unwrap_or(20.0);
    if now - start > warmup {
        stats.samples.push(time.delta_secs() * 1000.0);
    }
    if now - start > warmup + span {
        let mut v = stats.samples.clone();
        v.sort_by(|a, b| a.total_cmp(b));
        let n = v.len().max(1);
        let mean = v.iter().sum::<f32>() / n as f32;
        let pct = |p: f32| v.get(((n as f32 - 1.0) * p) as usize).copied().unwrap_or(0.0);
        let mut out = format!(
            "frames={} mean_ms={mean:.3} p50_ms={:.3} p95_ms={:.3} p99_ms={:.3} fps={:.1} rss_mib={:.0}\n",
            v.len(),
            pct(0.5),
            pct(0.95),
            pct(0.99),
            1000.0 / mean.max(1e-3),
            rss_mib()
        );
        if let Some(d) = diagnostics {
            for diag in d.iter() {
                let p = diag.path().as_str();
                if p.starts_with("render/") || p.contains("elapsed_gpu") {
                    if let Some(avg) = diag.average() {
                        out.push_str(&format!("diag {p} avg={avg:.4}\n"));
                    }
                }
            }
        }
        let _ = std::fs::write(&path, out);
        exit.write(AppExit::Success);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_switches_are_all_off() {
        let s = ReflectionSwitches::parse("");
        assert!(!s.any());
    }

    #[test]
    fn switch_list_parses() {
        let s = ReflectionSwitches::parse("env,glass=trans,mirror,mirror_size=128x64,mirror_every=3,wet");
        assert!(s.env && s.mirror && s.wet && !s.ssr);
        assert_eq!(s.glass, GlassMode::Transmission);
        assert_eq!(s.mirror_size, UVec2::new(128, 64));
        assert_eq!(s.mirror_every, 3);
    }

    #[test]
    fn wet_standard_is_identity_when_dry_and_darker_when_wet() {
        let (c, r) = wet_standard(Color::linear_rgb(0.5, 0.5, 0.5), 0.8, 0.0);
        assert!((c.to_linear().red - 0.5).abs() < 1e-6 && (r - 0.8).abs() < 1e-6);
        let (c, r) = wet_standard(Color::linear_rgb(0.5, 0.5, 0.5), 0.8, 1.0);
        assert!(c.to_linear().red < 0.5 && r < 0.8);
    }

    #[test]
    fn names_are_classified() {
        assert!(is_glass_name("scheibe_front.tga"));
        assert!(is_mirror_name("spiegel.bmp"));
        assert!(!is_glass_name("karosserie.bmp"));
    }

    #[test]
    fn f16_encodes_one() {
        assert_eq!(f16_bits(1.0), 0x3c00);
        assert_eq!(f16_bits(0.0), 0);
    }
}
