//! Puddles on roads and pavements while it rains.
//!
//! The terrain has puddles in its shader (`ground_layers.wesl`); the splines (roads, pavements,
//! parking bays) are plain materials and only get darker and smoother when wet. This module adds
//! the puddles there: flat, irregular, nearly mirror-smooth patches of dark water laid on the
//! road surface around the camera, as many as the weather's puddle coverage and the wetness ask
//! for, so a wet street shows patches that reflect the sky and the sun's glint at grazing angles
//! (roughness 0.03, the sky cube of `opts_bevy::reflections` as the environment).
//!
//! * **Where.** Random points in a ring around the camera ([`MIN_DISTANCE_M`]..[`MAX_DISTANCE_M`]);
//!   a point counts when the [`GroundIndex`] says a road lies there and the road is level across
//!   the patch (within [`FLAT_TOLERANCE_M`] over its extent), so patches never float on a slope.
//! * **How many.** [`puddle_target`]: [`MAX_PUDDLES`] at full coverage and wetness, none when it
//!   is dry or snow covers the ground. At most [`SPAWN_PER_TICK`] appear per tick; puddles left
//!   behind farther than [`DROP_DISTANCE_M`] and those above the target are removed.
//! * **Intensity.** The rain intensity setting (`graphics.rain_intensity`) scales coverage and
//!   wetness, so more rain means more puddles.

use std::f32::consts::TAU;

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use opts_bevy::reflections::Wetness;
use opts_bevy::weather::SurfaceState;
use opts_bevy::{SessionEnvironment, WeatherEffects};

use crate::in_game::omsi_world::GroundIndex;
use crate::in_game::types::{InGameCamera, InGameEntity};

use super::surfaces::wanted_state;

/// Most puddles at once.
pub const MAX_PUDDLES: usize = 80;
/// Nearest a puddle is placed to the camera (m).
pub const MIN_DISTANCE_M: f32 = 4.0;
/// Farthest a puddle is placed from the camera (m).
pub const MAX_DISTANCE_M: f32 = 55.0;
/// A puddle farther than this from the camera is removed (m).
pub const DROP_DISTANCE_M: f32 = 75.0;
/// Seconds between two ticks.
const TICK_S: f32 = 0.25;
/// Most puddles added per tick.
pub const SPAWN_PER_TICK: usize = 3;
/// How much the road may rise or fall across a puddle (m).
pub const FLAT_TOLERANCE_M: f32 = 0.02;
/// How far above the road a puddle lies (m).
const LIFT_M: f32 = 0.012;
/// Brightness (nits) of the sky colour the water shows at a full-bright sky.
const SKY_NITS: f32 = 1.2;
/// Tries to find a place per tick.
const TRIES_PER_TICK: usize = 24;

/// Marks a puddle.
#[derive(Component, Debug, Clone, Copy)]
pub struct Puddle {
    /// Position in the ground index frame (east, north).
    pub at: Vec2,
}

/// The puddles of the drive.
#[derive(Resource)]
pub struct PuddleState {
    mesh: Option<Handle<Mesh>>,
    material: Option<Handle<StandardMaterial>>,
    timer: f32,
    rng: u64,
}

impl Default for PuddleState {
    fn default() -> Self {
        Self {
            mesh: None,
            material: None,
            timer: 0.0,
            rng: 0x5EED_1234_ABCD_9876,
        }
    }
}

impl PuddleState {
    fn next(&mut self) -> f32 {
        // xorshift64*
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        let v = self.rng.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (v >> 40) as f32 / 16_777_216.0
    }
}

/// How many puddles the surface state asks for: coverage (`0..=0.45`) over its full value times
/// the wetness, times [`MAX_PUDDLES`].
#[must_use]
pub fn puddle_target(state: &SurfaceState, wet: f32) -> usize {
    let coverage = (state.puddles / 0.45).clamp(0.0, 1.0);
    (coverage * wet.clamp(0.0, 1.0) * MAX_PUDDLES as f32).round() as usize
}

/// The opacity of a puddle's picture at `(u, v)` in `0..1`: an irregular blob, opaque in the
/// middle and soft at the edge.
#[must_use]
pub fn blob_alpha(u: f32, v: f32) -> f32 {
    let (x, y) = (2.0 * u - 1.0, 2.0 * v - 1.0);
    let d = x.hypot(y);
    let a = y.atan2(x);
    // The edge of the blob wobbles with the angle.
    let edge = 0.78 + 0.1 * (3.0 * a + 0.7).sin() + 0.06 * (5.0 * a + 2.1).sin();
    let t = ((edge - d) / 0.28).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}


/// The shared plane and the dark water material, made on first use.
fn ensure_assets(
    state: &mut PuddleState,
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
) {
    if state.mesh.is_none() {
        state.mesh = Some(meshes.add(Plane3d::default().mesh().size(1.0, 1.0)));
        let blob = images.add(super::precip::texture(64, 64, blob_alpha));
        state.material = Some(materials.add(StandardMaterial {
            base_color: Color::linear_rgba(0.012, 0.016, 0.02, 0.92),
            base_color_texture: Some(blob),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.03,
            reflectance: 0.9,
            cull_mode: None,
            double_sided: true,
            // Roads draw with a depth bias of 4: above them.
            depth_bias: 8.0,
            ..default()
        }));
    }
}

/// Tries to place new puddles around the camera on level road, up to the per-tick limit and the
/// target.
fn spawn_new(
    commands: &mut Commands,
    state: &mut PuddleState,
    ground: &GroundIndex,
    (camera, centre): (Vec3, Vec2),
    (mesh, material, count, target): (Handle<Mesh>, Handle<StandardMaterial>, usize, usize),
) {
    let mut added = 0;
    for _ in 0..TRIES_PER_TICK {
        if added >= SPAWN_PER_TICK || count + added >= target {
            break;
        }
        let angle = state.next() * TAU;
        let distance = MIN_DISTANCE_M + state.next() * (MAX_DISTANCE_M - MIN_DISTANCE_M);
        let at = centre + Vec2::new(angle.cos(), angle.sin()) * distance;
        let hint = camera.y - 1.4;
        let Some(z) = ground.road_height_at(at.x, at.y, hint) else {
            continue;
        };
        let (sx, sz) = (0.9 + state.next() * 2.6, 0.7 + state.next() * 1.8);
        let yaw = state.next() * TAU;
        // Level across the patch: the road at the ends of both axes is as high as at the middle.
        let (sin, cos) = yaw.sin_cos();
        let level = [(sx, 0.0), (-sx, 0.0), (0.0, sz), (0.0, -sz)]
            .iter()
            .all(|(dx, dy)| {
                let p = at + Vec2::new(cos * dx - sin * dy, sin * dx + cos * dy) * 0.5;
                ground
                    .road_height_at(p.x, p.y, z)
                    .is_some_and(|h| (h - z).abs() <= FLAT_TOLERANCE_M)
            });
        if !level {
            continue;
        }
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform {
                translation: Vec3::new(at.x, z + LIFT_M, -at.y),
                rotation: Quat::from_rotation_y(yaw),
                scale: Vec3::new(sx, 1.0, sz),
            },
            NotShadowCaster,
            NotShadowReceiver,
            Puddle { at },
            InGameEntity,
            Name::new("Puddle"),
        ));
        added += 1;
    }
}

/// Adds, keeps and removes the puddles.
#[allow(clippy::too_many_arguments)]
pub fn update_puddles(
    time: Res<Time<Real>>,
    env: Res<SessionEnvironment>,
    effects: Res<WeatherEffects>,
    wetness: Res<Wetness>,
    ground: Option<Res<GroundIndex>>,
    cameras: Query<&GlobalTransform, With<InGameCamera>>,
    mut state: ResMut<PuddleState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    puddles: Query<(Entity, &Puddle)>,
    mut commands: Commands,
) {
    state.timer += time.delta_secs();
    if state.timer < TICK_S {
        return;
    }
    state.timer = 0.0;
    let surface = wanted_state(&env, &effects);
    let target = puddle_target(&surface, wetness.0);
    let camera = cameras.iter().next().map(|c| c.translation());
    let centre = camera.map(|c| Vec2::new(c.x, -c.z));
    // Remove the far ones and those above the target (the farthest first).
    let mut present: Vec<(Entity, f32)> = puddles
        .iter()
        .map(|(e, p)| (e, centre.map_or(0.0, |c| c.distance(p.at))))
        .collect();
    present.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut count = present.len();
    for (e, d) in &present {
        if count > target || *d > DROP_DISTANCE_M {
            commands.entity(*e).despawn();
            count -= 1;
        }
    }
    let (Some(ground), Some(camera), Some(centre)) = (ground, camera, centre) else {
        return;
    };
    if count >= target {
        return;
    }
    ensure_assets(&mut state, (&mut meshes, &mut materials, &mut images));
    let (Some(mesh), Some(material)) = (state.mesh.clone(), state.material.clone()) else {
        return;
    };
    // The water mirrors the sky: pale grey under cloud, bluer when it is clear, as bright as the
    // light (the cloud brightness follows the sun and the cover), not black blobs.
    let look = opts_bevy::weather::weather_look(env.conditions(), [0.5; 3], env.sun_alt_deg());
    let clear = (1.0 - look.cloud).clamp(0.0, 1.0);
    let sky = LinearRgba::rgb(0.8 - 0.25 * clear, 0.85 - 0.12 * clear, 0.95)
        * (SKY_NITS * look.cloud.max(0.15));
    if let Some(mut m) = materials.get_mut(&material) {
        if m.emissive != sky {
            m.emissive = sky;
        }
    }
    spawn_new(
        &mut commands,
        &mut state,
        &ground,
        (camera, centre),
        (mesh, material, count, target),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(puddles: f32) -> SurfaceState {
        SurfaceState {
            puddles,
            ..Default::default()
        }
    }

    #[test]
    fn the_target_follows_coverage_and_wetness() {
        assert_eq!(puddle_target(&state(0.0), 1.0), 0);
        assert_eq!(puddle_target(&state(0.45), 0.0), 0);
        assert_eq!(puddle_target(&state(0.45), 1.0), MAX_PUDDLES);
        assert_eq!(puddle_target(&state(0.225), 1.0), MAX_PUDDLES / 2);
        assert_eq!(puddle_target(&state(0.45), 0.5), MAX_PUDDLES / 2);
        // More than full coverage does not add more.
        assert_eq!(puddle_target(&state(0.9), 1.0), MAX_PUDDLES);
    }

    #[test]
    fn a_puddle_is_opaque_in_the_middle_and_gone_at_the_corners() {
        assert!(blob_alpha(0.5, 0.5) > 0.99);
        for (u, v) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
            assert!(blob_alpha(u, v) < 0.01, "({u}, {v})");
        }
        // Irregular: the edge is not a circle.
        let radii: Vec<f32> = (0..16)
            .map(|k| {
                let a = k as f32 / 16.0 * TAU;
                (0..200)
                    .map(|i| i as f32 / 200.0)
                    .take_while(|r| {
                        blob_alpha(0.5 + 0.5 * r * a.cos(), 0.5 + 0.5 * r * a.sin()) > 0.5
                    })
                    .last()
                    .unwrap_or(0.0)
            })
            .collect();
        let (lo, hi) = radii
            .iter()
            .fold((f32::MAX, f32::MIN), |(l, h), r| (l.min(*r), h.max(*r)));
        assert!(hi - lo > 0.1, "{radii:?}");
    }

    #[test]
    fn rain_intensity_raises_the_wetness_and_with_it_the_puddles() {
        use opts_bevy::weather::surface_state_for;
        let c = opts_bevy::WeatherConditions {
            ground_wet: [0.0, 120.0, 60.0],
            ..Default::default()
        };
        let half = surface_state_for(&c, 0.5, 1.0);
        let full = surface_state_for(&c, 1.0, 1.0);
        let heavy = surface_state_for(&c, 2.0, 1.0);
        assert!(half.puddles < full.puddles && full.puddles < heavy.puddles);
        assert!(half.wet < full.wet && full.wet < heavy.wet);
        assert_eq!(surface_state_for(&c, 0.0, 1.0).puddles, 0.0);
    }
}
