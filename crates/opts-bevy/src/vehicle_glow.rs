//! Light effects of vehicles (`[light_enh]`, `[light_enh_2]` in `model.cfg`) and a small pool of
//! real headlights.
//!
//! * **Glow sprites:** every light effect becomes a sprite under the vehicle's root (not under a
//!   LOD group, so it never pops with the meshes), drawn by the scenery light system
//!   ([`crate::scenery_runtime::scenery_light_system`]: size by brightness, fade by the cone).
//!   Brightness = variable × factor, clamped to `0..=1`, approached with the block's time
//!   constant. The variable comes from the vehicle's scripts or its AI variable table
//!   ([`VariableSource`]), so player and AI vehicles share the code.
//! * **Headlight pool:** forward-facing white light effects mark a vehicle's headlights. Of the
//!   vehicles whose headlights shine, the nearest [`HEADLIGHT_POOL`] get one real, shadowless
//!   `SpotLight` each (at the middle of the pair), with hysteresis so the choice is stable under
//!   small camera moves. Everything else is only a glow (hundreds of spot lights would cost too
//!   much).
//!
//! Design: `docs/src/architecture/ai-traffic-kinematics.md`, §5.

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use opts_core::formats::scenery::ScoLight;
use opts_core::formats::vehicles::model::ModelConfig;

use crate::ai_variables::{AiVariables, VariableSource};
use crate::coords::omsi_to_bevy_pos;
use crate::scenery_runtime::{SceneryLight, SceneryLightAssets, VarRef};
use crate::vehicle_script::VehicleScripts;

/// Number of real headlights (one spot light per vehicle).
pub const HEADLIGHT_POOL: usize = 6;
/// A vehicle keeps its spot light until it is this much farther than the nearest one without.
pub const HEADLIGHT_HYSTERESIS: f32 = 1.25;
/// Headlights shine (for the pool) above this brightness.
const HEADLIGHT_ON: f32 = 0.3;
/// Range (m) and intensity (lumens) of a pooled headlight.
const HEADLIGHT_RANGE_M: f32 = 60.0;
const HEADLIGHT_LUMENS: f32 = 60_000.0;

/// A light effect of a vehicle; the entity also carries the [`SceneryLight`] that is drawn.
#[derive(Component, Debug, Clone)]
pub struct VehicleGlow {
    /// The vehicle root whose scripts or AI variables drive the light.
    pub owner: Entity,
    /// Brightness variable (empty: always on).
    pub variable: String,
    /// Multiplier on the variable.
    pub factor: f32,
    /// Seconds to reach 63 % of a new brightness (`0` = at once).
    pub time_const_s: f32,
    /// Colour, 0–255.
    pub color: [f32; 3],
    /// The light is a headlight (shines forward, white).
    pub headlight: bool,
}

impl VehicleGlow {
    /// The brightness after `dt` seconds towards `target` from `level`.
    #[must_use]
    pub fn approach(&self, level: f32, target: f32, dt: f32) -> f32 {
        if self.time_const_s <= 1e-3 {
            target
        } else {
            level + (target - level) * (1.0 - (-dt / self.time_const_s).exp())
        }
    }
}

/// Whether a light effect is a headlight: directed forward (OMSI +y) and white-ish.
#[must_use]
pub fn is_headlight(light: &ScoLight) -> bool {
    !light.omni && light.direction.y > 0.7 && light.color.iter().all(|c| *c >= 150.0)
}

/// The light effects of `model` that belong to the meshes drawn up close: those outside any
/// `[LOD]` group and those of the detail group(s) ([`ModelConfig::detail_lod`]).
pub fn detail_lights(model: &ModelConfig) -> impl Iterator<Item = &ScoLight> {
    let first = model.lods.first().map(|l| l.threshold);
    let second = match (model.lods.first(), model.lods.get(1)) {
        (Some(a), Some(b)) if a.threshold <= 0.0 && b.threshold > 0.0 => Some(b.threshold),
        _ => None,
    };
    model.lights.iter().filter(move |l| {
        l.lod_threshold.is_none() || l.lod_threshold == first || l.lod_threshold == second
    })
}

/// Spawns the glow sprites of `model`'s light effects as children of `parent` (the vehicle unit
/// they sit on), driven by the variables of `owner` (the vehicle root). Mesh and material are
/// added by [`attach_glow_assets`]. Returns the sprite entities.
pub fn spawn_vehicle_glows(
    commands: &mut Commands,
    owner: Entity,
    parent: Entity,
    model: &ModelConfig,
) -> Vec<Entity> {
    let half = |deg: f32| (deg.clamp(0.0, 360.0) * 0.5).to_radians().cos();
    let mut out = Vec::new();
    for light in detail_lights(model) {
        let sprite = SceneryLight {
            var: VarRef::One,
            factor: light.factor,
            size_m: light.size_m.max(0.01),
            direction: if light.omni {
                Vec3::ZERO
            } else {
                omsi_to_bevy_pos(light.direction).normalize_or_zero()
            },
            cos_inner: half(light.cone_inner_deg),
            cos_outer: half(light.cone_outer_deg),
            level: 0.0,
        };
        let glow = VehicleGlow {
            owner,
            variable: light.variable.trim().to_owned(),
            factor: light.factor,
            time_const_s: light.time_const_s.max(0.0),
            color: light.color,
            headlight: is_headlight(light),
        };
        let e = commands
            .spawn((
                Transform::from_translation(omsi_to_bevy_pos(light.position))
                    .with_scale(Vec3::splat(sprite.size_m)),
                Visibility::Hidden,
                NotShadowCaster,
                NotShadowReceiver,
                sprite,
                glow,
                Name::new("VehicleGlow"),
            ))
            .id();
        commands.entity(parent).add_child(e);
        out.push(e);
    }
    out
}

/// Shared sprite mesh and per-colour materials of vehicle glows.
#[derive(Resource, Default)]
pub struct VehicleGlowAssets(pub SceneryLightAssets);

/// Gives new glow sprites their mesh and material.
pub fn attach_glow_assets(
    mut commands: Commands,
    fresh: Query<(Entity, &VehicleGlow), Without<Mesh3d>>,
    mut assets: ResMut<VehicleGlowAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (e, glow) in &fresh {
        let mesh = assets.0.sphere(&mut meshes);
        let material = assets.0.material(&mut materials, glow.color);
        commands
            .entity(e)
            .insert((Mesh3d(mesh), MeshMaterial3d(material)));
    }
}

/// Sets each glow's brightness from its owner's variable, smoothed by its time constant.
pub fn vehicle_glow_system(
    time: Res<Time>,
    vehicles: Query<(Option<&VehicleScripts>, Option<&AiVariables>)>,
    mut glows: Query<(&VehicleGlow, &mut SceneryLight)>,
) {
    let dt = time.delta_secs().min(0.5);
    for (glow, mut light) in &mut glows {
        let Ok((scripts, table)) = vehicles.get(glow.owner) else {
            continue;
        };
        let value = if glow.variable.is_empty() {
            1.0
        } else {
            let Some(source) = VariableSource::of(scripts, table) else {
                continue;
            };
            source.get(&glow.variable)
        };
        let target = (value * glow.factor).clamp(0.0, 1.0);
        let level = glow.approach(light.level, target, dt);
        let level = if (level - target).abs() < 1e-3 {
            target
        } else {
            level
        };
        if light.level != level {
            light.level = level;
        }
    }
}

/// The vehicles that get a real headlight: the `n` nearest of `candidates` (vehicle, distance),
/// except that a vehicle in `current` keeps its light while it is no farther than
/// `hysteresis` × the distance of the nearest candidate left out.
#[must_use]
pub fn choose_headlights(
    candidates: &[(Entity, f32)],
    current: &[Entity],
    n: usize,
    hysteresis: f32,
) -> Vec<Entity> {
    let mut sorted = candidates.to_vec();
    sorted.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    let mut chosen: Vec<(Entity, f32)> = sorted.iter().take(n).copied().collect();
    // An incumbent left out displaces the farthest newcomer when it is nearly as near.
    for inc in sorted.iter().filter(|c| current.contains(&c.0)) {
        if chosen.iter().any(|c| c.0 == inc.0) {
            continue;
        }
        let newcomer = chosen
            .iter()
            .enumerate()
            .filter(|(_, c)| !current.contains(&c.0))
            .max_by(|a, b| a.1 .1.total_cmp(&b.1 .1))
            .map(|(k, c)| (k, c.1));
        match newcomer {
            Some((k, d)) if inc.1 <= d * hysteresis => chosen[k] = *inc,
            _ => {}
        }
    }
    chosen.into_iter().map(|c| c.0).collect()
}

/// A pooled real headlight and the vehicle it belongs to.
#[derive(Component, Debug, Default)]
pub struct PooledHeadlight {
    /// The vehicle root it lights for, if any.
    pub owner: Option<Entity>,
}

/// Assigns the pooled spot lights to the nearest vehicles whose headlights shine and keeps them
/// in front of those vehicles.
pub fn headlight_pool_system(
    mut commands: Commands,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    glows: Query<(&VehicleGlow, &SceneryLight, &GlobalTransform)>,
    owners: Query<&GlobalTransform, Without<PooledHeadlight>>,
    mut pool: Query<(
        Entity,
        &mut PooledHeadlight,
        &mut Transform,
        &mut Visibility,
        &mut SpotLight,
    )>,
) {
    let Some(cam) = cameras
        .iter()
        .find(|(c, _)| c.is_active)
        .map(|(_, t)| t.translation())
    else {
        return;
    };
    // Shining headlights per vehicle: the middle of the pair and its forward direction.
    let mut per_owner: Vec<(Entity, Vec3, Vec3, usize, [f32; 3])> = Vec::new();
    for (glow, light, global) in &glows {
        if !glow.headlight || light.level < HEADLIGHT_ON {
            continue;
        }
        let pos = global.translation();
        let dir = global.rotation() * light.direction;
        match per_owner.iter_mut().find(|p| p.0 == glow.owner) {
            Some(p) => {
                p.1 += pos;
                p.2 += dir;
                p.3 += 1;
            }
            None => per_owner.push((glow.owner, pos, dir, 1, glow.color)),
        }
    }
    let candidates: Vec<(Entity, f32)> = per_owner
        .iter()
        .map(|p| (p.0, (p.1 / p.3 as f32).distance(cam)))
        .collect();
    let current: Vec<Entity> = pool.iter().filter_map(|p| p.1.owner).collect();
    let chosen = choose_headlights(&candidates, &current, HEADLIGHT_POOL, HEADLIGHT_HYSTERESIS);
    // Grow the pool lazily.
    let size = pool.iter().count();
    for _ in size..HEADLIGHT_POOL.min(chosen.len().max(size)) {
        commands.spawn((
            SpotLight {
                range: HEADLIGHT_RANGE_M,
                intensity: HEADLIGHT_LUMENS,
                inner_angle: 12f32.to_radians(),
                outer_angle: 30f32.to_radians(),
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::default(),
            Visibility::Hidden,
            PooledHeadlight::default(),
            Name::new("PooledHeadlight"),
        ));
    }
    // Free lights whose vehicle was not chosen, then give the free ones to new vehicles.
    let mut unassigned: Vec<Entity> = chosen
        .iter()
        .copied()
        .filter(|e| !current.contains(e))
        .collect();
    for (_, mut slot, _, mut vis, _) in &mut pool {
        if slot.owner.is_some_and(|o| !chosen.contains(&o)) {
            slot.owner = None;
            *vis = Visibility::Hidden;
        }
    }
    for (_, mut slot, mut transform, mut vis, mut spot) in &mut pool {
        if slot.owner.is_none() {
            slot.owner = unassigned.pop();
        }
        let Some(owner) = slot.owner else {
            continue;
        };
        let Some(p) = per_owner.iter().find(|p| p.0 == owner) else {
            continue;
        };
        if owners.get(owner).is_err() {
            slot.owner = None;
            *vis = Visibility::Hidden;
            continue;
        }
        let pos = p.1 / p.3 as f32;
        let dir = p.2.normalize_or_zero();
        if dir == Vec3::ZERO {
            continue;
        }
        *transform = Transform::from_translation(pos).looking_to(dir, Vec3::Y);
        let c = p.4.map(|v| (v / 255.0).clamp(0.0, 1.0));
        spot.color = Color::srgb(c[0], c[1], c[2]);
        if *vis != Visibility::Inherited {
            *vis = Visibility::Inherited;
        }
    }
}

/// Registers the vehicle glow and headlight pool systems.
#[derive(Default)]
pub struct VehicleGlowPlugin;

impl Plugin for VehicleGlowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VehicleGlowAssets>().add_systems(
            Update,
            (
                attach_glow_assets,
                vehicle_glow_system.before(crate::scenery_runtime::SceneryRuntimeSet),
                headlight_pool_system,
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(i: u32) -> Entity {
        Entity::from_raw_u32(i).expect("valid index")
    }

    #[test]
    fn the_pool_takes_the_nearest_and_keeps_them_under_small_moves() {
        let cands: Vec<(Entity, f32)> = (1..=10).map(|i| (e(i), i as f32 * 10.0)).collect();
        let first = choose_headlights(&cands, &[], 6, HEADLIGHT_HYSTERESIS);
        assert_eq!(first, (1..=6).map(e).collect::<Vec<_>>());
        // Vehicle 7 comes a little nearer than vehicle 6: no swap (within the hysteresis).
        let mut moved = cands.clone();
        moved[6].1 = 58.0;
        let second = choose_headlights(&moved, &first, 6, HEADLIGHT_HYSTERESIS);
        assert!(second.contains(&e(6)) && !second.contains(&e(7)));
        // Much nearer: it takes over.
        moved[6].1 = 5.0;
        let third = choose_headlights(&moved, &second, 6, HEADLIGHT_HYSTERESIS);
        assert!(third.contains(&e(7)) && third.len() == 6);
        // Fewer candidates than lights.
        assert_eq!(choose_headlights(&cands[..2], &[], 6, 1.25).len(), 2);
    }

    #[test]
    fn brightness_approaches_with_the_time_constant() {
        let glow = VehicleGlow {
            owner: Entity::PLACEHOLDER,
            variable: "ai_light".into(),
            factor: 1.0,
            time_const_s: 0.2,
            color: [255.0; 3],
            headlight: true,
        };
        let after = glow.approach(0.0, 1.0, 0.2);
        assert!((after - (1.0 - (-1.0f32).exp())).abs() < 1e-5);
        let instant = VehicleGlow {
            time_const_s: 0.0,
            ..glow
        };
        assert_eq!(instant.approach(0.0, 1.0, 0.01), 1.0);
    }

    #[test]
    fn glows_sit_under_the_vehicle_root_and_follow_its_variables() {
        let text = "[LOD]\n0.1\n\n[mesh]\nbody.o3d\n\n[light_enh_2]\n0\n2\n0.5\n0\n1\n0\n0\n0\n1\n0\n0\n255\n255\n255\n0.3\n40\n80\nAI_Light\n1\n0\n0\n0\n0\n\n[LOD]\n0.02\n\n[mesh]\nlow.o3d\n\n[light_enh_2]\n0\n2\n0.5\n0\n1\n0\n0\n0\n1\n0\n0\n255\n255\n255\n0.3\n40\n80\nAI_Light\n1\n0\n0\n0\n0\n";
        let model = ModelConfig::parse(text).expect("model");
        assert_eq!(model.lights.len(), 2);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, vehicle_glow_system);
        let root = app.world_mut().spawn(AiVariables::default()).id();
        let glows = {
            let mut commands = app.world_mut().commands();
            spawn_vehicle_glows(&mut commands, root, root, &model)
        };
        app.world_mut().flush();
        assert_eq!(glows.len(), 1, "only the detail LOD's light");
        assert_eq!(
            app.world().get::<ChildOf>(glows[0]).map(ChildOf::parent),
            Some(root)
        );
        assert!(app.world().get::<VehicleGlow>(glows[0]).unwrap().headlight);
        app.world_mut()
            .get_mut::<AiVariables>(root)
            .unwrap()
            .set("ai_light", 1.0);
        app.update();
        assert_eq!(
            app.world().get::<SceneryLight>(glows[0]).unwrap().level,
            1.0
        );
    }
}
