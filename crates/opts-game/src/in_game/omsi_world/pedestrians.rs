//! Pedestrians walking on the footpaths of the loaded tiles.
//!
//! Lanes come from the type-1 `[path]` records of spline profiles and scenery objects
//! ([`opts_sim::pedestrian`]); characters are the `.hum` files of the map's `humans.txt`, built
//! with the same rig as the player ([`opts_bevy::spawn_human`]). See
//! `docs/src/simulation/pedestrians.md`.

use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use opts_bevy::coords::{omsi_euler_to_bevy_quat, omsi_to_bevy_pos};
use opts_bevy::{pose_rotations, rest_pivots, spawn_human, Human, HumanPose, ModelPart};
use opts_core::formats::humans::HumanProfile;
use opts_core::formats::map::{MapObjectInstance, MapTerrain, MapTile};
use opts_sim::pedestrian::{
    gait, lanes_from_object, lanes_from_spline, Advance, FootLane, PedestrianNet, Walker,
};

use crate::in_game::types::InGameEntity;

use super::vehicle::load_human_parts;
use super::WorldLoadJob;

/// Default cap on simultaneously living pedestrians.
pub const DEFAULT_MAX_PEDESTRIANS: usize = 40;
/// One pedestrian per this much footpath (m) in the loaded tiles, up to the cap.
const METRES_OF_PATH_PER_PEDESTRIAN: f32 = 80.0;
/// Pedestrians appear at least this far from the camera (m)…
const SPAWN_MIN_M: f32 = 25.0;
/// …and at most this far.
const SPAWN_MAX_M: f32 = 140.0;
/// Pedestrians farther than this are removed (m).
const DESPAWN_M: f32 = 175.0;
/// Seconds between spawn attempts.
const SPAWN_INTERVAL_S: f32 = 0.3;

/// The footpath network of the resident tiles.
#[derive(Resource, Default)]
pub struct PedestrianNetwork(pub PedestrianNet);

/// Pedestrian settings.
#[derive(Resource, Clone, Copy)]
pub struct PedestrianSettings {
    /// Cap on living pedestrians (`0` disables them).
    pub max: usize,
}

impl Default for PedestrianSettings {
    fn default() -> Self {
        Self {
            max: DEFAULT_MAX_PEDESTRIANS,
        }
    }
}

/// A loaded character model.
struct Variant {
    name: String,
    profile: HumanProfile,
    parts: Vec<ModelPart>,
}

/// Character models of the map's `humans.txt`, loaded on demand.
#[derive(Resource, Default)]
pub struct PedestrianPool {
    /// Entries of `humans.txt` (duplicates weight the draw).
    entries: Vec<String>,
    /// Distinct entries and their loaded model, load attempts so far.
    slots: Vec<(String, Option<Variant>, u32)>,
    started: bool,
    rng: u32,
    frames: u32,
}

/// Component of a living pedestrian.
#[derive(Component)]
pub struct Pedestrian {
    walker: Walker,
    leg_m: f32,
}

/// Footpath lanes of a tile in the world frame: splines (`profile.paths`) and the paths of the
/// placed scenery objects.
pub(super) fn tile_lanes(
    job: &mut WorldLoadJob,
    tile: &MapTile,
    attached: &[(MapObjectInstance, f32)],
    terrain: Option<&MapTerrain>,
    origin: (f32, f32),
) -> Vec<FootLane> {
    let mut out = Vec::new();
    for inst in &tile.splines {
        let Some(profile) = job.sli(&inst.sli_file) else {
            continue;
        };
        if profile.paths.iter().any(|p| p.path_type == 1) {
            out.extend(lanes_from_spline(inst, &profile, origin));
        }
    }
    let placed = tile
        .objects
        .iter()
        .map(|o| (o, None))
        .chain(attached.iter().map(|(o, z)| (o, Some(*z))));
    for (obj, attached_z) in placed {
        let Some((_, sco)) = job.sco(&obj.sco_file) else {
            continue;
        };
        if !sco.paths.iter().any(|p| p.path_type == 1) {
            continue;
        }
        let absolute = sco.has_absolute_height();
        let ground = if absolute {
            0.0
        } else {
            terrain.map_or(0.0, |t| {
                opts_bevy::sample_terrain(t, obj.position.x, obj.position.y)
            })
        };
        let z = match (attached_z, absolute) {
            (Some(z), true) => z,
            _ => ground + obj.position.z,
        };
        let world = Vec3::new(origin.0 + obj.position.x, origin.1 + obj.position.y, z);
        out.extend(lanes_from_object(&sco.paths, world, obj.heading_deg));
    }
    out
}

/// Registers a tile's lanes with the network.
pub(super) fn register_tile(commands: &mut Commands, key: (i32, i32), lanes: Vec<FootLane>) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<PedestrianNetwork>() {
            net.0.insert_tile(key, lanes);
        }
    });
}

/// Removes an unloaded tile's lanes.
pub(super) fn unregister_tile(commands: &mut Commands, key: (i32, i32)) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<PedestrianNetwork>() {
            net.0.remove_tile(key);
        }
    });
}

/// Reads `humans.txt` once and loads one missing character model every few frames.
pub fn load_variants(
    job: Option<ResMut<WorldLoadJob>>,
    settings: Res<PedestrianSettings>,
    mut pool: ResMut<PedestrianPool>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(mut job) = job else { return };
    if settings.max == 0 {
        return;
    }
    if !pool.started {
        pool.started = true;
        pool.rng = 0x9E37_79B9;
        let text = job
            .install
            .find_in(&job.config.map_dir, "humans.txt")
            .and_then(|p| std::fs::read(p).ok())
            .map(|b| opts_core::formats::text::decode_bytes(&b))
            .unwrap_or_default();
        pool.entries = text
            .lines()
            .map(str::trim)
            .filter(|l| l.to_ascii_lowercase().ends_with(".hum"))
            .map(str::to_string)
            .collect();
        let mut distinct: Vec<String> = Vec::new();
        for e in &pool.entries {
            if !distinct.iter().any(|d| d.eq_ignore_ascii_case(e)) {
                distinct.push(e.clone());
            }
        }
        pool.slots = distinct.into_iter().map(|d| (d, None, 0)).collect();
        if pool.slots.is_empty() {
            tracing::error!(at = %opts_bevy::position_tag(), map = %job.config.map_dir.display(), "no characters in humans.txt; no pedestrians");
        }
    }
    pool.frames += 1;
    if !pool.frames.is_multiple_of(20) {
        return;
    }
    let Some(i) = pool
        .slots
        .iter()
        .position(|(_, v, tries)| v.is_none() && *tries < 40)
    else {
        return;
    };
    pool.slots[i].2 += 1;
    let rel = pool.slots[i].0.clone();
    let Some(path) = job.install.find_file(&rel) else {
        tracing::error!(at = %opts_bevy::position_tag(), hum = %rel, "pedestrian character not found");
        pool.slots[i].2 = 40;
        return;
    };
    let profile = match std::fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|b| HumanProfile::parse(&b).map_err(|e| e.to_string()))
    {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), hum = %path.display(), "pedestrian profile failed: {e}");
            pool.slots[i].2 = 40;
            return;
        }
    };
    let dir = path
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();
    super::trace::begin_frame();
    let loaded_parts = load_human_parts(
        &mut job,
        &mut meshes,
        &mut materials,
        &mut images,
        &profile,
        &dir,
    );
    let trace_frame = super::trace::end_frame();
    match loaded_parts {
        Ok(parts)
            if parts.iter().any(|p| {
                meshes
                    .get(&p.mesh)
                    .is_some_and(|m| m.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_none())
            }) =>
        {
            // Without bone weights the mesh cannot follow the skeleton (and a skinned draw call
            // with a non-skinned mesh is a render validation error).
            tracing::error!(at = %opts_bevy::position_tag(), hum = %path.display(), "character mesh has no bone weights; not used as pedestrian");
            pool.slots[i].2 = 40;
        }
        Ok(parts) => {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            tracing::info!(character = %name, parts = parts.len(), "pedestrian model loaded");
            super::trace::remember_human(&name, &trace_frame);
            pool.slots[i].1 = Some(Variant {
                name,
                profile,
                parts,
            });
        }
        Err(e) => {
            // Scrambled meshes decode in the background; try again later.
            tracing::debug!(hum = %path.display(), "pedestrian model not ready: {e}");
        }
    }
}

/// Spawns pedestrians on lanes around the camera up to the density-derived target.
#[allow(clippy::too_many_arguments)]
pub fn spawn_pedestrians(
    time: Res<Time>,
    mut elapsed: Local<f32>,
    mut commands: Commands,
    net: Res<PedestrianNetwork>,
    settings: Res<PedestrianSettings>,
    mut pool: ResMut<PedestrianPool>,
    mut bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    living: Query<(), With<Pedestrian>>,
) {
    *elapsed += time.delta_secs();
    if *elapsed < SPAWN_INTERVAL_S || settings.max == 0 {
        return;
    }
    *elapsed = 0.0;
    let Ok(cam) = camera.single() else { return };
    let target =
        ((net.0.total_length() / METRES_OF_PATH_PER_PEDESTRIAN) as usize).min(settings.max);
    if living.iter().count() >= target {
        return;
    }
    // Draw a character, weighted by its share of the `humans.txt` lines.
    let loaded: Vec<usize> = pool
        .slots
        .iter()
        .enumerate()
        .filter(|(_, (_, v, _))| v.is_some())
        .map(|(i, _)| i)
        .collect();
    if loaded.is_empty() {
        return;
    }
    let centre = (cam.translation().x, -cam.translation().z);
    let mut rng = pool.rng;
    let Some((lane, s)) = net
        .0
        .random_spot(centre, SPAWN_MIN_M, SPAWN_MAX_M, &mut rng)
    else {
        pool.rng = rng;
        return;
    };
    let weights: Vec<usize> = loaded
        .iter()
        .map(|&i| {
            pool.entries
                .iter()
                .filter(|e| e.eq_ignore_ascii_case(&pool.slots[i].0))
                .count()
                .max(1)
        })
        .collect();
    let mut pick = (opts_sim::pedestrian::net::next_f32(&mut rng)
        * weights.iter().sum::<usize>() as f32) as usize;
    let mut chosen = loaded[0];
    for (k, w) in weights.iter().enumerate() {
        if pick < *w {
            chosen = loaded[k];
            break;
        }
        pick -= w;
    }
    // Children and the very old walk more slowly.
    let age = pool.slots[chosen]
        .1
        .as_ref()
        .and_then(|v| v.profile.age_years);
    let base = match age {
        Some(a) if a < 14 => 1.0,
        Some(a) if a > 70 => 0.9,
        _ => 1.25,
    };
    let speed = base + 0.3 * opts_sim::pedestrian::net::next_f32(&mut rng);
    let walker = Walker::new(&net.0, lane, s, speed, &mut rng);
    pool.rng = rng;
    let Some(walker) = walker else { return };
    let Some(v) = pool.slots[chosen].1.as_ref() else {
        return;
    };
    let leg_m = rest_pivots(&v.profile)[0].y.max(0.3);
    let root = spawn_human(
        &mut commands,
        &mut bindposes,
        &v.profile,
        &v.parts,
        &v.name,
        HumanPose::Standing,
    );
    if opts_bevy::scene_trace::enabled() {
        let frame = super::trace::human_frame(&v.name);
        let tf = Transform::from_translation(
            walker
                .pose(&net.0)
                .map(|(p, _)| omsi_to_bevy_pos(p))
                .unwrap_or_default(),
        );
        super::trace::spawned(
            opts_bevy::scene_trace::kind::HUMAN,
            &v.name,
            &v.name,
            root,
            &tf,
            &frame,
            "ok",
            serde_json::json!({}),
        );
    }
    commands.entity(root).insert((
        Pedestrian { walker, leg_m },
        Visibility::default(),
        InGameEntity,
    ));
}

/// Walks the pedestrians, poses them and removes finished or distant ones.
pub fn move_pedestrians(
    time: Res<Time>,
    mut commands: Commands,
    net: Res<PedestrianNetwork>,
    mut pool: ResMut<PedestrianPool>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut peds: Query<(Entity, &mut Pedestrian, &Human)>,
    mut transforms: Query<&mut Transform>,
) {
    let dt = time.delta_secs().min(0.1);
    let cam = camera.single().ok().map(|c| c.translation());
    let mut rng = pool.rng ^ 0x5bd1_e995;
    for (entity, mut ped, human) in &mut peds {
        let outcome = ped.walker.advance(dt, &net.0, &mut rng);
        let Some((pos, heading)) = ped.walker.pose(&net.0) else {
            commands.entity(entity).despawn();
            continue;
        };
        let bevy_pos = omsi_to_bevy_pos(pos);
        let far = cam.is_some_and(|c| c.distance(bevy_pos) > DESPAWN_M);
        if outcome == Advance::Finished || far {
            commands.entity(entity).despawn();
            continue;
        }
        if let Ok(mut t) = transforms.get_mut(entity) {
            t.translation = bevy_pos;
            t.rotation = omsi_euler_to_bevy_quat(-heading.to_degrees(), 0.0, 0.0);
        }
        // Gait.
        let walk = human.profile.walk;
        let stride = walk.stride_m.max(0.3);
        ped.walker.phase = (ped.walker.phase
            + std::f32::consts::TAU * ped.walker.speed / stride * dt)
            % std::f32::consts::TAU;
        let g = gait(&walk, ped.leg_m, ped.walker.phase);
        let mut q = pose_rotations(&human.profile, HumanPose::Standing);
        q[3] = Quat::from_rotation_x(g.thigh[0]);
        q[4] = Quat::from_rotation_x(g.thigh[1]);
        q[5] = Quat::from_rotation_x(-g.knee[0]);
        q[6] = Quat::from_rotation_x(-g.knee[1]);
        q[7] = Quat::from_rotation_x(g.arm[0]) * q[7];
        q[8] = Quat::from_rotation_x(g.arm[1]) * q[8];
        q[1] = Quat::from_rotation_y(g.waist_twist);
        for (i, joint) in human.rig.joints.iter().enumerate() {
            if let Ok(mut t) = transforms.get_mut(*joint) {
                t.rotation = q[i];
                if i == 0 {
                    let rest = rest_pivots(&human.profile)[0];
                    t.translation = rest + Vec3::new(g.hip_shift, -g.bob, 0.0);
                }
            }
        }
    }
    pool.rng = rng;
}

/// Reads `--pedestrians` and resets the per-session state when a session starts.
pub fn configure_pedestrians(
    args: Option<Res<crate::AppArgs>>,
    user: Option<Res<crate::settings::GameSettings>>,
    mut settings: ResMut<PedestrianSettings>,
    mut net: ResMut<PedestrianNetwork>,
    mut pool: ResMut<PedestrianPool>,
    mut ground: ResMut<super::GroundIndex>,
) {
    *ground = super::GroundIndex::default();
    // The command line wins for this run; then the setting *Pedestrians*; then the default.
    settings.max = args
        .and_then(|a| a.0.pedestrians)
        .or_else(|| user.and_then(|u| u.raw("gameplay.pedestrians")?.parse().ok()))
        .unwrap_or(DEFAULT_MAX_PEDESTRIANS);
    net.0 = PedestrianNet::default();
    *pool = PedestrianPool::default();
}
