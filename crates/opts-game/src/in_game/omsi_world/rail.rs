//! Rail-bound vehicles in the game: the track network of the loaded tiles, the player's tram and
//! AI trains. The physics live in [`opts_sim::rail`]; see `docs/src/simulation/rail-vehicles.md`.

use std::path::Path;

use bevy::prelude::*;
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::VehicleController;
use opts_core::formats::map::{MapObjectInstance, MapTerrain, MapTile};
use opts_core::formats::scenery::LaneKind;
use opts_core::formats::vehicles::model::{ModelConfig, ModelLod};
use opts_core::formats::vehicles::BusManifest;
use opts_sim::pedestrian::{
    lanes_from_object_of_type, lanes_of_kind, FootLane, PedestrianNet, Travel,
};
use opts_sim::rail::{Branch, Moved, RailDynamics, RailInput, Train};
use opts_sim::GearPosition;

use crate::in_game::types::InGameEntity;

use super::vehicle::load_segment_meshes;
use super::WorldLoadJob;

/// `[path]` type of rail.
const PATH_TYPE_RAIL: u32 = 2;
/// A player vehicle spawned farther than this from any track is not put on rails (m).
const SNAP_MAX_M: f32 = 12.0;
/// Default number of AI trains.
pub const DEFAULT_MAX_TRAINS: usize = 3;
/// Cruise speed of AI trains (m/s).
const AI_CRUISE_MS: f32 = 9.0;
/// AI trains appear this far from the camera (m)…
const AI_SPAWN_MIN_M: f32 = 80.0;
/// …up to this far…
const AI_SPAWN_MAX_M: f32 = 400.0;
/// …and are removed beyond this (m).
const AI_DESPAWN_M: f32 = 600.0;

/// The track network of the resident tiles.
#[derive(Resource, Default)]
pub struct RailNetwork(pub PedestrianNet);

/// Number of AI trains (`--trains N`).
#[derive(Resource, Clone, Copy)]
pub struct RailSettings {
    /// Cap on AI trains.
    pub max_trains: usize,
}

impl Default for RailSettings {
    fn default() -> Self {
        Self {
            max_trains: DEFAULT_MAX_TRAINS,
        }
    }
}

// --- network ----------------------------------------------------------------------------------

/// Track lanes of a tile in the session frame: splines (rail `[path]` of their profile) and the
/// rail paths of placed objects (points, crossings, junction plates).
pub(super) fn tile_rail_lanes(
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
        if profile.paths.iter().any(|p| p.path_type == PATH_TYPE_RAIL) {
            let name = inst.sli_file.rsplit(['\\', '/']).next().unwrap_or("");
            out.extend(lanes_of_kind(inst, &profile, name, origin, LaneKind::Rail));
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
        if !sco.paths.iter().any(|p| p.path_type == PATH_TYPE_RAIL) {
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
        out.extend(lanes_from_object_of_type(
            &sco.paths,
            world,
            obj.heading_deg,
            PATH_TYPE_RAIL,
        ));
    }
    out
}

/// Registers a tile's track lanes.
pub(super) fn register_tile(commands: &mut Commands, key: (i32, i32), lanes: Vec<FootLane>) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<RailNetwork>() {
            net.0.insert_tile(key, lanes);
        }
    });
}

/// Removes an unloaded tile's track lanes.
pub(super) fn unregister_tile(commands: &mut Commands, key: (i32, i32)) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<RailNetwork>() {
            net.0.remove_tile(key);
        }
    });
}

/// Resets the per-session state and reads `--trains`.
pub fn configure_rail(
    args: Option<Res<crate::AppArgs>>,
    mut settings: ResMut<RailSettings>,
    mut net: ResMut<RailNetwork>,
    mut pool: ResMut<RailAiPool>,
) {
    settings.max_trains = args.and_then(|a| a.0.trains).unwrap_or(DEFAULT_MAX_TRAINS);
    net.0 = PedestrianNet::default();
    *pool = RailAiPool::default();
}

// --- player ------------------------------------------------------------------------------------

/// A player vehicle that runs on rails (`[type] 2`): the chassis is held on the track and its
/// speed comes from [`RailDynamics`] instead of the road drivetrain.
#[derive(Component)]
pub struct RailPlayer {
    train: Option<Train>,
    speed: f32,
    dynamics: RailDynamics,
    pitch: f32,
    gave_up: bool,
}

impl RailPlayer {
    /// A tram of `mass_kg` waiting for its track.
    #[must_use]
    pub fn new(mass_kg: f32) -> Self {
        Self {
            train: None,
            speed: 0.0,
            dynamics: RailDynamics::tram(mass_kg),
            pitch: 0.0,
            gave_up: false,
        }
    }
}

/// Whether a manifest is a rail vehicle.
#[must_use]
pub fn is_rail(manifest: &BusManifest) -> bool {
    manifest.vehicle_type == Some(2)
}

/// Holds the player's tram on the track and runs its longitudinal dynamics (fixed step, after the
/// road physics, before the articulation of the following units).
pub fn rail_player_step(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    net: Res<RailNetwork>,
    mut players: Query<(&mut VehicleController, &mut RailPlayer)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for (mut controller, mut rail) in &mut players {
        if rail.gave_up {
            continue;
        }
        let (traction, brake, hold) = {
            let i = &controller.sim.inputs;
            (
                if i.gear() == GearPosition::Drive {
                    i.throttle()
                } else {
                    0.0
                },
                i.service_brake(),
                i.is_parking_brake_engaged(),
            )
        };
        let ch = &mut controller.sim.chassis;
        if rail.train.is_none() {
            if net.0.lane_count() == 0 {
                continue; // tiles still streaming in
            }
            let p = Vec3::new(ch.position.x, ch.position.y, ch.elevation_z);
            match net.0.nearest(p, SNAP_MAX_M) {
                Some((lane, s, dist)) => {
                    let l = net.0.lane(lane).expect("lane exists");
                    let (_, h) = l.at(s);
                    let along = (h - ch.heading_rad).cos();
                    let dir = if along >= 0.0 { 1 } else { -1 };
                    tracing::info!(lane, s, dist, dir, "rail vehicle placed on the track");
                    rail.train = Some(Train::new(lane, s, dir));
                }
                None => {
                    tracing::error!(
                        at = %opts_bevy::position_tag(),
                        "rail vehicle spawned more than {SNAP_MAX_M} m from any track; it stays a road vehicle"
                    );
                    rail.gave_up = true;
                    continue;
                }
            }
        }
        let input = RailInput {
            traction,
            brake,
            emergency: false,
            hold,
            direction: 1,
        };
        let Some(train) = rail.train.as_mut() else {
            continue;
        };
        // Route choice at the next point: Z left, X straight, C right.
        if keyboard.just_pressed(KeyCode::KeyZ) {
            train.branch = Branch::Left;
        } else if keyboard.just_pressed(KeyCode::KeyX) {
            train.branch = Branch::Straight;
        } else if keyboard.just_pressed(KeyCode::KeyC) {
            train.branch = Branch::Right;
        }
        let pose_ahead = train.pose(&net.0);
        let grade = match (pose_ahead, train.pose_behind(&net.0, 6.0)) {
            (Some((a, _)), Some((b, _))) => (a.z - b.z) / 6.0,
            _ => 0.0,
        };
        let speed = rail.dynamics.step(rail.speed, grade, &input, dt);
        rail.speed = speed;
        let Some(train) = rail.train.as_mut() else {
            continue;
        };
        if train.advance(speed * dt, &net.0) == Moved::EndOfTrack {
            rail.speed = 0.0;
        }
        let Some(train) = rail.train.as_ref() else {
            continue;
        };
        if let Some((p, h)) = train.pose(&net.0) {
            ch.position = Vec2::new(p.x, p.y);
            ch.heading_rad = h;
            ch.elevation_z = p.z;
            ch.local_velocity = Vec2::new(0.0, rail.speed);
            ch.yaw_rate_rad_s = 0.0;
        }
        if let Some((b, _)) = train.pose_behind(&net.0, 6.0) {
            let z = ch.elevation_z;
            rail.pitch = ((z - b.z) / 6.0).atan();
        }
    }
}

/// Pitches the player's tram along the gradient (after the transform sync of the vehicle).
pub fn rail_player_pose(mut players: Query<(&RailPlayer, &VehicleController, &mut Transform)>) {
    for (rail, controller, mut tf) in &mut players {
        if rail.train.is_none() {
            continue;
        }
        tf.rotation = Quat::from_rotation_y(-controller.sim.chassis.heading_rad)
            * Quat::from_rotation_x(rail.pitch);
    }
}

// --- AI trains ---------------------------------------------------------------------------------

/// One unit of a consist.
struct UnitSpec {
    name: String,
    lod: ModelLod,
    model_dir: std::path::PathBuf,
    tex_dirs: Vec<std::path::PathBuf>,
    /// Distance of the unit's origin behind the head unit's origin (m).
    behind: f32,
}

/// A trainset assembled from a `.bus`/`.ovh` chain.
struct TrainSpec {
    name: String,
    units: Vec<UnitSpec>,
}

/// Rail vehicles found in the map's AI lists.
#[derive(Resource, Default)]
pub struct RailAiPool {
    started: bool,
    specs: Vec<TrainSpec>,
    elapsed: f32,
    rng: u32,
}

/// A running AI train.
#[derive(Component)]
pub struct AiTrain {
    train: Train,
    speed: f32,
    units: Vec<(Entity, f32)>,
    wheel_angle: f32,
}

/// Reads the AI lists once and keeps every rail vehicle (`[type] 2`) with its coupling chain.
pub fn load_rail_variants(job: Option<ResMut<WorldLoadJob>>, mut pool: ResMut<RailAiPool>) {
    let Some(job) = job else { return };
    if pool.started {
        return;
    }
    pool.started = true;
    pool.rng = 0x7A41_C0DE;
    let Some(path) = job.install.find_in(&job.config.map_dir, "ailists.cfg") else {
        return;
    };
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let lists = opts_core::formats::map::AiLists::parse(&bytes);
    let mut seen = std::collections::HashSet::new();
    for g in &lists.groups {
        for e in &g.entries {
            let f = e.vehicle.to_ascii_lowercase();
            if !(f.ends_with(".bus") || f.ends_with(".ovh")) || !seen.insert(f) {
                continue;
            }
            if let Some(spec) = load_spec(&job, &e.vehicle) {
                pool.specs.push(spec);
            }
        }
    }
    tracing::info!(trains = pool.specs.len(), "rail vehicles in the AI lists");
}

fn parse_unit(
    job: &WorldLoadJob,
    path: &Path,
) -> Option<(
    BusManifest,
    ModelLod,
    std::path::PathBuf,
    Vec<std::path::PathBuf>,
)> {
    let manifest = BusManifest::parse_bytes(&std::fs::read(path).ok()?).ok()?;
    let dir = path.parent()?.to_path_buf();
    let cfg = job.install.find_in(&dir, manifest.model_cfg.as_deref()?)?;
    let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).ok()?).ok()?;
    let lod = model
        .lods
        .iter()
        .find(|l| l.threshold == 0.0)
        .or_else(|| model.lods.first())?
        .clone();
    let model_dir = cfg.parent()?.to_path_buf();
    let tex_dirs = vec![
        job.install
            .find_in(&dir, "texture")
            .unwrap_or_else(|| dir.join("texture")),
        model_dir.join("texture"),
    ];
    Some((manifest, lod, model_dir, tex_dirs))
}

/// The coupling chain of a rail vehicle; `None` for road vehicles and unreadable files.
fn load_spec(job: &WorldLoadJob, rel: &str) -> Option<TrainSpec> {
    let path = job.install.find_file(rel)?;
    let (mut manifest, lod, model_dir, tex_dirs) = parse_unit(job, &path)?;
    if !is_rail(&manifest) {
        return None;
    }
    let name = path.file_stem()?.to_string_lossy().into_owned();
    let mut units = vec![UnitSpec {
        name: name.clone(),
        lod,
        model_dir,
        tex_dirs,
        behind: 0.0,
    }];
    let mut dir = path.parent()?.to_path_buf();
    let mut behind = 0.0_f32;
    for _ in 0..12 {
        let Some(next) = manifest.couple_back.clone() else {
            break;
        };
        let Some(next_path) = job.install.find_in(&dir, &next) else {
            break;
        };
        let Some((m, lod, model_dir, tex_dirs)) = parse_unit(job, &next_path) else {
            break;
        };
        let (Some(b), Some(f)) = (manifest.coupling_back, m.coupling_front) else {
            break;
        };
        // The next unit's origin lies at `b − f` in this unit's frame (y forward).
        behind += -(b.y - f.y);
        units.push(UnitSpec {
            name: next_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            lod,
            model_dir,
            tex_dirs,
            behind,
        });
        dir = next_path.parent()?.to_path_buf();
        manifest = m;
    }
    Some(TrainSpec { name, units })
}

fn next_rand(state: &mut u32) -> f32 {
    opts_sim::pedestrian::net::next_f32(state)
}

/// Spawns AI trains on tracks around the camera up to `--trains`.
#[allow(clippy::too_many_arguments)]
pub fn spawn_ai_trains(
    time: Res<Time>,
    mut commands: Commands,
    job: Option<ResMut<WorldLoadJob>>,
    settings: Res<RailSettings>,
    net: Res<RailNetwork>,
    mut pool: ResMut<RailAiPool>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    living: Query<(), With<AiTrain>>,
) {
    pool.elapsed += time.delta_secs();
    if pool.elapsed < 1.0 || settings.max_trains == 0 || pool.specs.is_empty() {
        return;
    }
    pool.elapsed = 0.0;
    let (Some(mut job), Ok(cam)) = (job, camera.single()) else {
        return;
    };
    if living.iter().count() >= settings.max_trains || net.0.lane_count() == 0 {
        return;
    }
    let centre = (cam.translation().x, -cam.translation().z);
    let mut rng = pool.rng;
    let spot = net
        .0
        .random_spot(centre, AI_SPAWN_MIN_M, AI_SPAWN_MAX_M, &mut rng);
    let Some((lane, s)) = spot else {
        pool.rng = rng;
        return;
    };
    let pick = (next_rand(&mut rng) * pool.specs.len() as f32) as usize % pool.specs.len();
    let dir = match net.0.lane(lane).map(|l| l.travel) {
        Some(Travel::Reverse) => -1,
        Some(Travel::Both) if next_rand(&mut rng) < 0.5 => -1,
        _ => 1,
    };
    pool.rng = rng;
    let spec = &pool.specs[pick];
    let mut units = Vec::new();
    for u in &spec.units {
        let root = commands
            .spawn((
                Transform::default(),
                Visibility::Hidden,
                Name::new(u.name.clone()),
                InGameEntity,
                opts_bevy::AiVariables::default(),
            ))
            .id();
        let (count, ..) = load_segment_meshes(
            &mut job,
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut images,
            root,
            &u.lod,
            &u.model_dir,
            &u.tex_dirs,
        );
        units.push((root, u.behind, count));
    }
    if units.iter().any(|u| u.2 == 0) {
        // Meshes still decoding in the background: discard and try again on the next tick.
        for (e, ..) in &units {
            commands.entity(*e).despawn();
        }
        return;
    }
    tracing::info!(train = %spec.name, units = spec.units.len(), lane, "AI train spawned");
    let head = units[0].0;
    commands.entity(head).insert(AiTrain {
        train: Train::new(lane, s, dir),
        speed: 0.0,
        units: units.iter().map(|u| (u.0, u.1)).collect(),
        wheel_angle: 0.0,
    });
}

/// Runs the AI trains along their track and poses every unit by its distance behind the head.
pub fn move_ai_trains(
    time: Res<Time>,
    mut commands: Commands,
    net: Res<RailNetwork>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut trains: Query<(Entity, &mut AiTrain)>,
    mut units: Query<(&mut Transform, &mut Visibility, &mut opts_bevy::AiVariables)>,
) {
    let dt = time.delta_secs().min(0.1);
    let cam = camera.single().ok().map(|c| c.translation());
    for (head, mut t) in &mut trains {
        t.speed = (t.speed + 0.8 * dt).min(AI_CRUISE_MS);
        let step = t.speed * dt;
        let end = t.train.advance(step, &net.0) == Moved::EndOfTrack;
        let far = cam.is_some_and(|c| {
            t.train
                .pose(&net.0)
                .is_some_and(|(p, _)| c.distance(omsi_to_bevy_pos(p)) > AI_DESPAWN_M)
        });
        if end || far || t.train.pose(&net.0).is_none() {
            for (e, _) in &t.units {
                commands.entity(*e).despawn();
            }
            let _ = head;
            continue;
        }
        t.wheel_angle += step / 0.29;
        let angle = t.wheel_angle;
        let list: Vec<(Entity, f32)> = t.units.clone();
        for (e, behind) in list {
            let Some((p, h)) = t.train.pose_behind(&net.0, behind) else {
                continue;
            };
            if let Ok((mut tf, mut vis, mut vars)) = units.get_mut(e) {
                tf.translation = omsi_to_bevy_pos(p);
                tf.rotation = Quat::from_rotation_y(-h);
                *vis = Visibility::Visible;
                for axle in 0..6 {
                    for side in ["l", "r"] {
                        vars.set(&format!("wheel_rotation_{axle}_{side}"), angle);
                    }
                }
            }
        }
    }
}
