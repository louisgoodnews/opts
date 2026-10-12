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
/// Cruise speed of cyclists (m/s).
const BIKE_CRUISE_MS: f32 = 4.5;
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

/// The cycle-path network of the resident tiles (`LaneKind::Bike` lanes of splines).
#[derive(Resource, Default)]
pub struct BikeNetwork(pub PedestrianNet);

/// Cap on riders (`--cyclists N`).
#[derive(Resource, Clone, Copy)]
pub struct BikeSettings {
    /// Cap on simultaneously riding cyclists.
    pub max_riders: usize,
}

impl Default for BikeSettings {
    fn default() -> Self {
        Self { max_riders: 6 }
    }
}

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

/// Cycle-path lanes of a tile's splines in the session frame.
pub(super) fn tile_bike_lanes(
    job: &mut WorldLoadJob,
    tile: &MapTile,
    origin: (f32, f32),
) -> Vec<FootLane> {
    let mut out = Vec::new();
    for inst in &tile.splines {
        let Some(profile) = job.sli(&inst.sli_file) else {
            continue;
        };
        let name = inst.sli_file.rsplit(['\\', '/']).next().unwrap_or("");
        out.extend(lanes_of_kind(inst, &profile, name, origin, LaneKind::Bike));
    }
    out
}

/// Registers a tile's cycle-path lanes.
pub(super) fn register_bike_tile(commands: &mut Commands, key: (i32, i32), lanes: Vec<FootLane>) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<BikeNetwork>() {
            net.0.insert_tile(key, lanes);
        }
    });
}

/// Removes an unloaded tile's cycle-path lanes.
pub(super) fn unregister_bike_tile(commands: &mut Commands, key: (i32, i32)) {
    commands.queue(move |world: &mut World| {
        if let Some(mut net) = world.get_resource_mut::<BikeNetwork>() {
            net.0.remove_tile(key);
        }
    });
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
    mut bike_settings: ResMut<BikeSettings>,
    mut bike_net: ResMut<BikeNetwork>,
    mut bike_pool: ResMut<BikeAiPool>,
) {
    bike_settings.max_riders = args
        .as_ref()
        .and_then(|a| a.0.cyclists)
        .unwrap_or(BikeSettings::default().max_riders);
    bike_net.0 = PedestrianNet::default();
    *bike_pool = BikeAiPool::default();
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
    /// Half bogie span of the head: the train point is its front bogie, its origin lies this far
    /// behind it.
    span: f32,
    gave_up: bool,
}

impl RailPlayer {
    /// A tram of `mass_kg` waiting for its track.
    #[must_use]
    pub fn new(mass_kg: f32, span: f32) -> Self {
        Self {
            train: None,
            speed: 0.0,
            dynamics: RailDynamics::tram(mass_kg),
            span,
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
                    let mut train = Train::new(lane, s, dir);
                    // The chassis position is the body's origin; the train point (front bogie)
                    // lies one half span ahead of it.
                    train.advance(rail.span, &net.0);
                    rail.train = Some(train);
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
        // The chassis is the head body: midpoint and heading of its two bogies on the track.
        if let Some((mid, heading, _)) = unit_frame(train, &net.0, rail.span, rail.span) {
            ch.position = Vec2::new(mid.x, mid.y);
            ch.heading_rad = heading;
            ch.elevation_z = mid.z;
            ch.local_velocity = Vec2::new(0.0, rail.speed);
            ch.yaw_rate_rad_s = 0.0;
        }
    }
}

/// One sub part of a composed vehicle (a car, a bogie, an articulation joint, a coupler).
#[derive(Debug, Clone)]
pub struct Part {
    /// The entity carrying the part's meshes and transform (the root for the head part).
    pub entity: Entity,
    /// Name of the part (its `.bus` file name; the head: the vehicle's model name).
    pub name: String,
    /// Distance of the part's origin behind the head's origin along the vehicle axis (m).
    pub behind: f32,
    /// Half distance between the part's bogies (`[boogies]`), 0 when unknown.
    pub half_span: f32,
    /// The part hangs backwards (coupled with the `true` flag).
    pub reversed: bool,
    /// Meshes spawned for the part (0 for the head, which is counted with the vehicle).
    pub meshes: usize,
    /// Vertices of the part's meshes.
    pub vertices: usize,
}

/// The sub parts of a composed vehicle, resolved once at spawn: the head and the coupled units
/// behind it in order. Lookups by name or index need no `[couple_back]` walk or file access.
#[derive(Component, Debug, Clone)]
pub struct VehicleComposition {
    /// The leading unit (the vehicle's root entity).
    pub head: Part,
    /// The coupled units behind the head, front to back.
    pub followers: Vec<Part>,
}

impl VehicleComposition {
    /// All parts, head first.
    pub fn parts(&self) -> impl Iterator<Item = &Part> {
        std::iter::once(&self.head).chain(self.followers.iter())
    }

    /// The first part with this name (case-insensitive, with or without `.bus`/`.ovh`).
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&Part> {
        let bare = |n: &str| {
            n.trim_end_matches(".bus")
                .trim_end_matches(".ovh")
                .to_string()
        };
        let want = bare(name);
        self.parts()
            .find(|p| bare(&p.name).eq_ignore_ascii_case(&want))
    }

    /// Number of parts including the head.
    #[must_use]
    pub fn len(&self) -> usize {
        1 + self.followers.len()
    }

    /// Always false: a composition has a head.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Distance from the head's origin to the origin of the last part (m).
    #[must_use]
    pub fn length(&self) -> f32 {
        self.followers.last().map_or(0.0, |p| p.behind)
    }
}

/// Smallest half span used for the chord of a unit without bogie data (m).
const MIN_HALF_SPAN: f32 = 1.0;

/// Frame of a unit whose origin is `centre` metres behind the train's track point: its two bogies
/// (`±half_span` about the origin) sit on the track; the origin is the midpoint, the heading the
/// chord between them and the pitch the gradient along it. Without bogie data a 2 m chord is used.
/// Returns the OMSI position, heading and pitch.
fn unit_frame(
    train: &Train,
    net: &PedestrianNet,
    centre: f32,
    half_span: f32,
) -> Option<(Vec3, f32, f32)> {
    let span = if half_span > 0.0 {
        half_span
    } else {
        MIN_HALF_SPAN
    };
    let front = train.pose_behind(net, (centre - span).max(0.0))?.0;
    let rear = train.pose_behind(net, centre + span)?.0;
    let mid = (front + rear) * 0.5;
    let (dx, dy) = (front.x - rear.x, front.y - rear.y);
    let run = dx.hypot(dy);
    if run > 0.1 {
        Some((mid, dx.atan2(dy), ((front.z - rear.z) / run).atan()))
    } else {
        Some((mid, train.pose_behind(net, centre)?.1, 0.0))
    }
}

/// [`unit_frame`] as a transform (OMSI to Bevy), turned half a circle for a reversed unit.
fn unit_pose(
    train: &Train,
    net: &PedestrianNet,
    centre: f32,
    half_span: f32,
    reversed: bool,
) -> Option<Transform> {
    let (mid, heading, pitch) = unit_frame(train, net, centre, half_span)?;
    let turn = if reversed { std::f32::consts::PI } else { 0.0 };
    Some(Transform {
        translation: omsi_to_bevy_pos(mid),
        rotation: Quat::from_rotation_y(-heading)
            * Quat::from_rotation_x(pitch)
            * Quat::from_rotation_y(turn),
        scale: Vec3::ONE,
    })
}

/// Places the player's tram (head and following units) on the track, and writes the joint angles
/// the models' bellows read.
pub fn rail_player_pose(
    net: Res<RailNetwork>,
    mut players: Query<(
        &RailPlayer,
        &mut Transform,
        Option<&VehicleComposition>,
        Option<&opts_bevy::VehicleScripts>,
    )>,
    mut units: Query<&mut Transform, Without<RailPlayer>>,
) {
    for (rail, mut tf, layout, scripts) in &mut players {
        let Some(train) = rail.train.as_ref() else {
            continue;
        };
        let head_span = rail.span;
        if let Some(t) = unit_pose(train, &net.0, head_span, head_span, false) {
            *tf = t;
        }
        let Some(layout) = layout else { continue };
        let mut previous = tf.rotation.to_euler(EulerRot::YXZ).0;
        let mut alphas = Vec::with_capacity(layout.followers.len());
        for part in &layout.followers {
            let (e, reversed) = (part.entity, part.reversed);
            let Some(pose) = unit_pose(
                train,
                &net.0,
                part.behind + head_span,
                part.half_span,
                reversed,
            ) else {
                continue;
            };
            let yaw = pose.rotation.to_euler(EulerRot::YXZ).0;
            let own = if reversed {
                yaw + std::f32::consts::PI
            } else {
                yaw
            };
            let d = (own - previous + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            // Bevy yaw is counter-clockwise; the models expect clockwise degrees.
            alphas.push(-d.to_degrees());
            previous = own;
            if let Ok(mut u) = units.get_mut(e) {
                *u = pose;
            }
        }
        if let Some(scripts) = scripts {
            if let Ok(mut engine) = scripts.engine.lock() {
                for (k, a) in alphas.iter().enumerate() {
                    engine.set(&format!("articulation_{k}_alpha"), *a);
                    engine.set(&format!("articulation_{k}_beta"), 0.0);
                    engine.set(&format!("articulation_{k}_gamma"), 0.0);
                }
            }
        }
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
    /// Hangs backwards (coupled with the `true` flag).
    reversed: bool,
    /// Half bogie span (`[boogies]`; 0 when unknown).
    span: f32,
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

/// Riders found in the map's AI lists (light `[type] 2` vehicles: bicycles, e-scooters).
#[derive(Resource, Default)]
pub struct BikeAiPool(RailAiPool);

/// A running AI train.
#[derive(Component)]
pub struct AiTrain {
    bike: bool,
    train: Train,
    speed: f32,
    composition: VehicleComposition,
    /// Half bogie span of the head unit.
    wheel_angle: f32,
}

/// Reads the AI lists once and keeps every rail vehicle (`[type] 2`) with its coupling chain.
pub fn load_rail_variants(
    job: Option<ResMut<WorldLoadJob>>,
    mut pool: ResMut<RailAiPool>,
    mut bikes: ResMut<BikeAiPool>,
) {
    let Some(job) = job else { return };
    if pool.started {
        return;
    }
    pool.started = true;
    pool.rng = 0x7A41_C0DE;
    bikes.0.started = true;
    bikes.0.rng = 0xB1CE_0001;
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
            if let Some((spec, light)) = load_spec(&job, &e.vehicle) {
                if light {
                    bikes.0.specs.push(spec);
                } else {
                    pool.specs.push(spec);
                }
            }
        }
    }
    tracing::info!(
        trains = pool.specs.len(),
        riders = bikes.0.specs.len(),
        "rail vehicles and cycles in the AI lists"
    );
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

/// The coupling chain of a `[type] 2` vehicle and whether it is a light one (bicycle, e-scooter:
/// under a tonne; OMSI files them under the rail type too); `None` for road vehicles and
/// unreadable files.
fn load_spec(job: &WorldLoadJob, rel: &str) -> Option<(TrainSpec, bool)> {
    let path = job.install.find_file(rel)?;
    let (mut manifest, lod, model_dir, tex_dirs) = parse_unit(job, &path)?;
    if !is_rail(&manifest) {
        return None;
    }
    let light = manifest.mass_tonnes.is_some_and(|m| m < 1.0);
    let name = path.file_stem()?.to_string_lossy().into_owned();
    let mut units = vec![UnitSpec {
        name: name.clone(),
        lod,
        model_dir,
        tex_dirs,
        behind: 0.0,
        reversed: false,
        span: manifest.bogie_half_span.unwrap_or(0.0),
    }];
    let mut dir = path.parent()?.to_path_buf();
    let mut behind = 0.0_f32;
    for _ in 0..40 {
        let Some(next) = manifest.couple_back.clone() else {
            break;
        };
        let Some(next_path) = job.install.find_in(&dir, &next) else {
            break;
        };
        let Some((m, lod, model_dir, tex_dirs)) = parse_unit(job, &next_path) else {
            break;
        };
        let reversed = manifest.couple_back_reversed;
        let own = if reversed {
            m.coupling_back.map(|c| -c)
        } else {
            m.coupling_front
        };
        let (Some(b), Some(f)) = (manifest.coupling_back, own) else {
            break;
        };
        // The next unit's origin lies at `b − f` in this unit's frame (y forward).
        behind += f.y - b.y;
        units.push(UnitSpec {
            name: next_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            lod,
            model_dir,
            tex_dirs,
            behind,
            reversed,
            span: m.bogie_half_span.unwrap_or(0.0),
        });
        if reversed {
            break; // the ring of a double-ended consist closes here
        }
        dir = next_path.parent()?.to_path_buf();
        manifest = m;
    }
    Some((TrainSpec { name, units }, light))
}

fn next_rand(state: &mut u32) -> f32 {
    opts_sim::pedestrian::net::next_f32(state)
}

/// Spawns AI trains on tracks and cyclists on cycle paths around the camera.
#[allow(clippy::too_many_arguments)]
pub fn spawn_ai_trains(
    time: Res<Time>,
    mut commands: Commands,
    job: Option<ResMut<WorldLoadJob>>,
    settings: Res<RailSettings>,
    bike_settings: Res<BikeSettings>,
    net: Res<RailNetwork>,
    bike_net: Res<BikeNetwork>,
    mut pool: ResMut<RailAiPool>,
    mut bike_pool: ResMut<BikeAiPool>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    living: Query<&AiTrain>,
) {
    let (Some(mut job), Ok(cam)) = (job, camera.single()) else {
        return;
    };
    let centre = (cam.translation().x, -cam.translation().z);
    let trains = living.iter().filter(|t| !t.bike).count();
    let riders = living.iter().count() - trains;
    let dt = time.delta_secs();
    let mut assets = (&mut *meshes, &mut *materials, &mut *images);
    try_spawn(
        false,
        dt,
        settings.max_trains.saturating_sub(trains),
        centre,
        &net.0,
        &mut pool,
        &mut job,
        &mut commands,
        &mut assets,
    );
    try_spawn(
        true,
        dt,
        bike_settings.max_riders.saturating_sub(riders),
        centre,
        &bike_net.0,
        &mut bike_pool.0,
        &mut job,
        &mut commands,
        &mut assets,
    );
}

type SpawnAssets<'a> = (
    &'a mut Assets<Mesh>,
    &'a mut Assets<StandardMaterial>,
    &'a mut Assets<Image>,
);

/// One spawn attempt per second and kind, when below the cap.
#[allow(clippy::too_many_arguments)]
fn try_spawn(
    bike: bool,
    dt: f32,
    room: usize,
    centre: (f32, f32),
    net: &PedestrianNet,
    pool: &mut RailAiPool,
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    assets: &mut SpawnAssets,
) {
    pool.elapsed += dt;
    if pool.elapsed < 1.0 || room == 0 || pool.specs.is_empty() || net.lane_count() == 0 {
        return;
    }
    pool.elapsed = 0.0;
    let (meshes, materials, images) = (&mut *assets.0, &mut *assets.1, &mut *assets.2);
    let mut rng = pool.rng;
    let spot = net.random_spot(centre, AI_SPAWN_MIN_M, AI_SPAWN_MAX_M, &mut rng);
    let Some((lane, s)) = spot else {
        pool.rng = rng;
        return;
    };
    let pick = (next_rand(&mut rng) * pool.specs.len() as f32) as usize % pool.specs.len();
    let dir = match net.lane(lane).map(|l| l.travel) {
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
            job,
            commands,
            meshes,
            materials,
            images,
            root,
            &u.lod,
            &u.model_dir,
            &u.tex_dirs,
        );
        units.push((root, u.behind, u.reversed, u.span, count));
    }
    if units.iter().any(|u| u.4 == 0) {
        // Meshes still decoding in the background: discard and try again on the next tick.
        for (e, ..) in &units {
            commands.entity(*e).despawn();
        }
        return;
    }
    tracing::info!(vehicle = %spec.name, units = spec.units.len(), lane, bike, "AI rail/cycle vehicle spawned");
    let head = units[0].0;
    let part = |i: usize, u: &(Entity, f32, bool, f32, usize)| Part {
        entity: u.0,
        name: spec.units[i].name.clone(),
        behind: u.1,
        half_span: u.3,
        reversed: u.2,
        meshes: u.4,
        vertices: 0,
    };
    let composition = VehicleComposition {
        head: part(0, &units[0]),
        followers: units
            .iter()
            .enumerate()
            .skip(1)
            .map(|(i, u)| part(i, u))
            .collect(),
    };
    commands.entity(head).insert(AiTrain {
        bike,
        train: Train::new(lane, s, dir),
        speed: 0.0,
        composition,
        wheel_angle: 0.0,
    });
}

/// Runs the AI trains along their track and poses every unit by its distance behind the head.
pub fn move_ai_trains(
    time: Res<Time>,
    mut commands: Commands,
    rail_net: Res<RailNetwork>,
    bike_net: Res<BikeNetwork>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut trains: Query<(Entity, &mut AiTrain)>,
    mut units: Query<(&mut Transform, &mut Visibility, &mut opts_bevy::AiVariables)>,
) {
    let dt = time.delta_secs().min(0.1);
    let cam = camera.single().ok().map(|c| c.translation());
    for (head, mut t) in &mut trains {
        let net = if t.bike { &bike_net.0 } else { &rail_net.0 };
        let (accel, cruise) = if t.bike {
            (1.0, BIKE_CRUISE_MS)
        } else {
            (0.8, AI_CRUISE_MS)
        };
        t.speed = (t.speed + accel * dt).min(cruise);
        let step = t.speed * dt;
        let end = t.train.advance(step, net) == Moved::EndOfTrack;
        let far = cam.is_some_and(|c| {
            t.train
                .pose(net)
                .is_some_and(|(p, _)| c.distance(omsi_to_bevy_pos(p)) > AI_DESPAWN_M)
        });
        if end || far || t.train.pose(net).is_none() {
            for p in t.composition.parts() {
                commands.entity(p.entity).despawn();
            }
            let _ = head;
            continue;
        }
        t.wheel_angle += step / 0.29;
        let angle = t.wheel_angle;
        let head_span = t.composition.head.half_span;
        let list: Vec<Part> = t.composition.parts().cloned().collect();
        for part in list {
            let (e, reversed) = (part.entity, part.reversed);
            let Some(pose) = unit_pose(
                &t.train,
                net,
                part.behind + head_span,
                part.half_span,
                reversed,
            ) else {
                continue;
            };
            if let Ok((mut tf, mut vis, mut vars)) = units.get_mut(e) {
                *tf = pose;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn part(name: &str, behind: f32) -> Part {
        Part {
            entity: Entity::PLACEHOLDER,
            name: name.into(),
            behind,
            half_span: 1.0,
            reversed: false,
            meshes: 1,
            vertices: 3,
        }
    }

    #[test]
    fn composition_finds_parts_by_name_and_reports_length() {
        let c = VehicleComposition {
            head: part("NF6D", 0.0),
            followers: vec![part("NF6D_C.bus", 12.5), part("NF6D_B_User.bus", 30.0)],
        };
        assert_eq!(c.len(), 3);
        assert!((c.length() - 30.0).abs() < 1e-6);
        assert_eq!(c.find("nf6d_c").map(|p| p.behind), Some(12.5));
        assert!(c.find("missing").is_none());
        assert_eq!(c.parts().count(), 3);
    }
}
