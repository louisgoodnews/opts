//! Rain and snow: one dynamic mesh of camera-facing quads around the camera, rebuilt every
//! frame, at most as many quads as the quality setting allows (`graphics.weather_precipitation`).
//!
//! A particle has no state: its place is a pure function of its fixed random seed and the time,
//! wrapped into a box around the camera, so particles stay where they are in the world while the
//! camera moves through them, and nothing has to be spawned or removed. Rain is drawn as streaks
//! along the fall direction (the wind slants it), snow as slowly falling flakes that flutter. The
//! effects use real time, so a preview on the paused *Environment* page keeps falling.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use opts_bevy::weather::{precip_plan, weather_look, PrecipPlan};
use opts_bevy::{SessionEnvironment, WeatherEffects};
use opts_core::formats::weather::PrecipKind;

use crate::in_game::types::{InGameCamera, InGameEntity};

/// Edge of the box (metres) the particles fill horizontally, centred on the camera.
pub const BOX_XZ: f32 = 44.0;
/// Height of the box; it reaches 8 m below and 12 m above the camera.
pub const BOX_Y: f32 = 20.0;
const BELOW_CAMERA: f32 = 8.0;

/// Marks the precipitation mesh.
#[derive(Component, Debug, Clone, Copy)]
pub struct PrecipLayer;

/// State of the precipitation.
#[derive(Resource, Default)]
pub struct PrecipState {
    /// Fixed random seeds `0..1` per particle (one per allocated quad).
    seeds: Vec<[f32; 3]>,
    /// Indices of all allocated quads.
    indices: Vec<u32>,
    mesh: Option<Handle<Mesh>>,
    rain: Option<Handle<StandardMaterial>>,
    snow: Option<Handle<StandardMaterial>>,
    /// Quads the mesh has vertices for.
    allocated: usize,
    /// Quads in the index buffer.
    drawn: usize,
    /// Kind the layer shows.
    kind: Option<PrecipKind>,
    /// Tint written to the materials last.
    tint: f32,
}

impl PrecipState {
    /// Quads the mesh is built for (the budget of the last plan).
    #[must_use]
    pub fn allocated(&self) -> usize {
        self.allocated
    }

    /// Quads that are drawn.
    #[must_use]
    pub fn drawn(&self) -> usize {
        self.drawn
    }

    /// The kind that is drawn, `None` while nothing falls.
    #[must_use]
    pub fn kind(&self) -> Option<PrecipKind> {
        self.kind
    }
}

fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

/// The fixed seed `0..1` of particle `i`.
#[must_use]
pub fn seed_of(i: u32) -> [f32; 3] {
    let f = |salt: u32| {
        (hash(i.wrapping_mul(3).wrapping_add(salt).wrapping_add(0x9e37)) >> 8) as f32 / 16_777_216.0
    };
    [f(0), f(1), f(2)]
}

/// Where a particle is at time `t` (seconds), for a camera at `camera`.
///
/// The particle falls with the plan's fall speed and drifts with its horizontal velocity; its
/// place is wrapped into the box around the camera. Snow flutters by a swing of 0.35 m.
#[must_use]
pub fn particle_position(seed: [f32; 3], t: f64, plan: &PrecipPlan, camera: Vec3) -> Vec3 {
    let wrap = |raw: f64, low: f64, len: f64| low + (raw - low).rem_euclid(len);
    let (lx, ly) = (f64::from(BOX_XZ), f64::from(BOX_Y));
    let x = wrap(
        f64::from(seed[0]) * lx + f64::from(plan.drift.x) * t,
        f64::from(camera.x) - lx / 2.0,
        lx,
    );
    let z = wrap(
        f64::from(seed[2]) * lx + f64::from(plan.drift.y) * t,
        f64::from(camera.z) - lx / 2.0,
        lx,
    );
    let y = wrap(
        f64::from(seed[1]) * ly - f64::from(plan.fall_speed) * t,
        f64::from(camera.y) - f64::from(BELOW_CAMERA),
        ly,
    );
    let mut p = Vec3::new(x as f32, y as f32, z as f32);
    if plan.kind == PrecipKind::Snow {
        let phase = f64::from(seed[0]) * std::f64::consts::TAU * 7.0;
        p.x += ((t * 0.8 + phase).sin() * 0.35) as f32;
        p.z += ((t * 0.7 + phase * 1.3).cos() * 0.35) as f32;
    }
    p
}

/// The four corners of a rain streak at `p`: a strip of the plan's size along the fall direction,
/// turned to face `camera`. Order: start left, start right, end right, end left.
#[must_use]
pub fn rain_corners(p: Vec3, plan: &PrecipPlan, camera: Vec3, fallback_right: Vec3) -> [Vec3; 4] {
    let axis = Vec3::new(plan.drift.x, -plan.fall_speed, plan.drift.y).normalize_or_zero();
    let side = axis
        .cross(p - camera)
        .try_normalize()
        .unwrap_or(fallback_right)
        * (plan.size.x * 0.5);
    let half = axis * (plan.size.y * 0.5);
    [
        p - half - side,
        p - half + side,
        p + half + side,
        p + half - side,
    ]
}

/// The four corners of a snow flake at `p`, facing the camera.
#[must_use]
pub fn flake_corners(p: Vec3, plan: &PrecipPlan, right: Vec3, up: Vec3) -> [Vec3; 4] {
    let (r, u) = (right * (plan.size.x * 0.5), up * (plan.size.y * 0.5));
    [p - r - u, p + r - u, p + r + u, p - r + u]
}

pub(super) fn texture(width: u32, height: u32, alpha: impl Fn(f32, f32) -> f32) -> Image {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let a = alpha(
                (x as f32 + 0.5) / width as f32,
                (y as f32 + 0.5) / height as f32,
            );
            data.extend([255, 255, 255, (a.clamp(0.0, 1.0) * 255.0) as u8]);
        }
    }
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn material(image: Handle<Image>, colour: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: colour,
        base_color_texture: Some(image),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    }
}

/// Colour of the particles for a scene-linear brightness `k` (`WeatherLook::precip`).
fn base_colour(kind: PrecipKind, k: f32) -> Color {
    match kind {
        PrecipKind::Snow => Color::linear_rgba(k, k, k, 0.9),
        _ => Color::linear_rgba(0.85 * k, 0.9 * k, k, 0.45),
    }
}

/// A mesh with vertices for `quads` quads (positions are written every frame) and its indices.
fn quad_mesh(quads: usize) -> (Mesh, Vec<u32>) {
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let indices: Vec<u32> = (0..quads as u32)
        .flat_map(|q| [0, 1, 2, 0, 2, 3].map(|i| q * 4 + i))
        .collect();
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; quads * 4])
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; quads * 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        (0..quads).flat_map(|_| uv).collect::<Vec<[f32; 2]>>(),
    )
    .with_inserted_indices(Indices::U32(indices[..0].to_vec()));
    (mesh, indices)
}

/// Materials and mesh, once; a changed budget builds the mesh again.
fn ensure_assets(
    state: &mut PrecipState,
    (images, meshes, materials): (
        &mut Assets<Image>,
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
    ),
    budget: usize,
) {
    if state.rain.is_none() {
        let streak = images.add(texture(4, 32, |u, v| {
            (1.0 - (2.0 * u - 1.0).abs()).powf(0.7) * (std::f32::consts::PI * v).sin().powf(1.4)
        }));
        let flake = images.add(texture(16, 16, |u, v| {
            let d = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() * 2.0;
            (1.0 - d).clamp(0.0, 1.0).powf(0.6)
        }));
        state.rain = Some(materials.add(material(streak, base_colour(PrecipKind::Rain, 1.0))));
        state.snow = Some(materials.add(material(flake, base_colour(PrecipKind::Snow, 1.0))));
    }
    if state.mesh.is_none() || state.allocated != budget {
        let (mesh, indices) = quad_mesh(budget);
        match state.mesh.clone() {
            Some(handle) => {
                let _ = meshes.insert(&handle, mesh);
            }
            None => state.mesh = Some(meshes.add(mesh)),
        }
        state.seeds = (0..budget as u32).map(seed_of).collect();
        state.indices = indices;
        state.allocated = budget;
        state.drawn = 0;
    }
}

/// Places the quads of the visible particles around the camera and updates the index range when
/// their number changed.
fn write_geometry(
    state: &mut PrecipState,
    meshes: &mut Assets<Mesh>,
    mesh_handle: &Handle<Mesh>,
    plan: &PrecipPlan,
    (t, cam): (f64, &GlobalTransform),
) {
    let quads = (plan.quads as usize).min(state.allocated);
    let drawn_changed = state.drawn != quads;
    let (right, up, cam_pos) = (cam.right().as_vec3(), cam.up().as_vec3(), cam.translation());
    let seeds = std::mem::take(&mut state.seeds);
    let indices = std::mem::take(&mut state.indices);
    if let Some(mut mesh) = meshes.get_mut(mesh_handle) {
        if let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
        {
            for (q, seed) in seeds.iter().take(quads).enumerate() {
                let p = particle_position(*seed, t, plan, cam_pos);
                let corners = if plan.kind == PrecipKind::Rain {
                    rain_corners(p, plan, cam_pos, right)
                } else {
                    flake_corners(p, plan, right, up)
                };
                for (k, c) in corners.iter().enumerate() {
                    positions[q * 4 + k] = c.to_array();
                }
            }
        }
        if drawn_changed {
            mesh.insert_indices(Indices::U32(indices[..quads * 6].to_vec()));
        }
    }
    state.seeds = seeds;
    state.indices = indices;
    state.drawn = quads;
}

/// Spawns the precipitation layer when it does not exist, shows it and gives it the material of
/// the current kind of precipitation.
fn sync_layer(
    commands: &mut Commands,
    layer: &mut Query<(&mut Visibility, &mut MeshMaterial3d<StandardMaterial>), With<PrecipLayer>>,
    mesh_handle: &Handle<Mesh>,
    want_material: &Handle<StandardMaterial>,
) {
    if layer.is_empty() {
        commands.spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(want_material.clone()),
            Transform::IDENTITY,
            Visibility::Visible,
            NoFrustumCulling,
            NotShadowCaster,
            NotShadowReceiver,
            PrecipLayer,
            InGameEntity,
            Name::new("Precipitation"),
        ));
    }
    for (mut v, mut m) in layer {
        if *v != Visibility::Visible {
            *v = Visibility::Visible;
        }
        if m.0 != *want_material {
            m.0 = want_material.clone();
        }
    }
}

/// Draws, updates and hides the precipitation.
#[allow(clippy::too_many_arguments)]
pub fn update_precipitation(
    time: Res<Time<Real>>,
    env: Res<SessionEnvironment>,
    effects: Res<WeatherEffects>,
    mut state: ResMut<PrecipState>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cameras: Query<&GlobalTransform, With<InGameCamera>>,
    mut layer: Query<(&mut Visibility, &mut MeshMaterial3d<StandardMaterial>), With<PrecipLayer>>,
    mut commands: Commands,
) {
    let plan = precip_plan(
        &effects.scaled(env.conditions()),
        effects.precipitation.budget(),
    );
    let Some(plan) = plan else {
        state.kind = None;
        for (mut v, _) in &mut layer {
            if *v != Visibility::Hidden {
                *v = Visibility::Hidden;
            }
        }
        return;
    };
    let Some(cam) = cameras.iter().next() else {
        return;
    };
    let budget = effects.precipitation.budget() as usize;

    ensure_assets(
        &mut state,
        (&mut images, &mut meshes, &mut materials),
        budget,
    );
    let mesh_handle = state.mesh.clone().unwrap_or_default();
    let want_material = if plan.kind == PrecipKind::Snow {
        state.snow.clone()
    } else {
        state.rain.clone()
    }
    .unwrap_or_default();

    sync_layer(&mut commands, &mut layer, &mesh_handle, &want_material);
    // Colour follows the daylight.
    let tint = weather_look(env.conditions(), [0.0; 3], env.sun_alt_deg()).precip;
    if state.kind != Some(plan.kind) || (state.tint - tint).abs() > 0.01 {
        state.tint = tint;
        for (handle, kind) in [
            (state.rain.clone(), PrecipKind::Rain),
            (state.snow.clone(), PrecipKind::Snow),
        ] {
            if let Some(mut m) = handle.and_then(|h| materials.get_mut(&h)) {
                m.base_color = base_colour(kind, tint);
            }
        }
    }
    state.kind = Some(plan.kind);

    // Geometry.
    write_geometry(
        &mut state,
        &mut meshes,
        &mesh_handle,
        &plan,
        (time.elapsed_secs_f64(), cam),
    );
}

#[cfg(test)]
mod tests;
