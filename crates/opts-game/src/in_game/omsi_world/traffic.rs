//! AI road traffic on the lanes of the loaded tiles (kinematic mode, M-0049 phase C).
//!
//! Lanes come from the type-0 `[path]` records of splines and scenery objects
//! ([`opts_traffic::segments_from_tile`]) and join geometrically; vehicles are drawn from the map's
//! `ailists.cfg` (group `NormalCars`, falling back to the first group) and driven by the
//! Intelligent Driver Model. The vehicle models are placed on the lane (no scripts, physics or
//! wheel spin yet). Plan and limits: `docs/src/architecture/ai-traffic.md`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_bevy::coords::{omsi_euler_to_bevy_quat, omsi_to_bevy_pos};
use opts_core::formats::map::lists::day;
use opts_core::formats::map::{
    parse_unsched_trafficdens, AiLists, GroupDensity, MapObjectInstance, MapTerrain, MapTile,
};
use opts_core::formats::scenery::sco::ScoPath;
use opts_core::formats::scenery::{SceneryObject, SplinePath, SplineProfile};
use opts_core::formats::text::{decode_bytes, scan_blocks};
use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::BusManifest;
use opts_traffic::{
    segments_from_tile, AgentId, ConnectParams, IdmParams, LaneGraph, LaneKind, LaneSegment,
    PathSource, Rng, TrafficWorld,
};

use crate::in_game::types::{InGameEntity, InGameSessionInfo};

use super::vehicle::load_segment_meshes;
use super::WorldLoadJob;

/// Default cap on simultaneously driving AI vehicles.
pub const DEFAULT_MAX_TRAFFIC: usize = 30;
/// One vehicle per this much road (m) in the loaded tiles at density factor 1, up to the cap.
const METRES_OF_ROAD_PER_VEHICLE: f64 = 250.0;
/// Vehicles appear at least this far from the camera (m)…
const SPAWN_MIN_M: f64 = 80.0;
/// …and at most this far.
const SPAWN_MAX_M: f64 = 220.0;
/// Vehicles farther than this are removed (m).
const DESPAWN_M: f32 = 320.0;
/// Seconds between spawn attempts.
const SPAWN_INTERVAL_S: f32 = 0.5;
/// Load attempts per vehicle type before it is given up (scrambled meshes decode in the background).
const MAX_LOAD_TRIES: u32 = 8;

/// Traffic settings.
#[derive(Resource, Clone, Copy)]
pub struct TrafficSettings {
    /// Cap on driving AI vehicles (`0` disables traffic).
    pub max: usize,
}

impl Default for TrafficSettings {
    fn default() -> Self {
        Self {
            max: DEFAULT_MAX_TRAFFIC,
        }
    }
}

/// The lane network of the resident tiles and the agents on it.
#[derive(Resource)]
pub struct TrafficState {
    world: TrafficWorld,
    /// Road lane segments per resident tile.
    segments: HashMap<(i32, i32), Vec<LaneSegment>>,
    /// The segments changed; the graph is rebuilt before the next step.
    dirty: bool,
    road_length_m: f64,
}

impl Default for TrafficState {
    fn default() -> Self {
        Self {
            world: TrafficWorld::new(LaneGraph::default(), 0x7AF1_C001),
            segments: HashMap::new(),
            dirty: false,
            road_length_m: 0.0,
        }
    }
}

/// A vehicle type of the map's AI list.
struct Variant {
    name: String,
    model: ModelConfig,
    model_dir: PathBuf,
    tex_dirs: Vec<PathBuf>,
    /// `[ai_deltaheight]`: vertical offset applied when driven by AI (m).
    delta_height: f32,
    /// Body length and the distance from the model origin to its front bumper (m), measured when
    /// the meshes first load.
    size: Option<(f32, f32)>,
}

enum SlotState {
    /// Not parsed yet.
    Pending,
    Ready(Box<Variant>),
    Failed,
}

struct Slot {
    file: String,
    weight: f32,
    /// Spawn attempts whose meshes were not ready (still decoding, or unreadable).
    fails: u32,
    state: SlotState,
}

/// Vehicle types of the map's `ailists.cfg`, loaded on demand.
#[derive(Resource, Default)]
pub struct TrafficPool {
    started: bool,
    slots: Vec<Slot>,
    density: Option<GroupDensity>,
    rng: Option<Rng>,
    frames: u32,
}

/// Component of a driving AI vehicle.
#[derive(Component)]
pub struct TrafficVehicle {
    id: AgentId,
    /// Distance from the model origin to the front bumper (m).
    front_offset: f32,
    delta_height: f32,
}

/// Path data for [`segments_from_tile`], read through the load job's caches.
struct JobSource<'a> {
    job: &'a mut WorldLoadJob,
    terrain: Option<&'a MapTerrain>,
    sli: HashMap<String, Option<Arc<SplineProfile>>>,
    sco: HashMap<String, Option<Arc<SceneryObject>>>,
}

impl PathSource for JobSource<'_> {
    fn sli_paths(&mut self, f: &str) -> Option<&[SplinePath]> {
        let job = &mut *self.job;
        self.sli
            .entry(f.to_owned())
            .or_insert_with(|| job.sli(f))
            .as_deref()
            .map(|p| p.paths.as_slice())
    }

    fn sco_paths(&mut self, f: &str) -> Option<&[ScoPath]> {
        let job = &mut *self.job;
        self.sco
            .entry(f.to_owned())
            .or_insert_with(|| job.sco(f).map(|(_, o)| o))
            .as_deref()
            .map(|o| o.paths.as_slice())
    }

    fn object_base_height(&mut self, o: &MapObjectInstance) -> f64 {
        let absolute = self
            .sco
            .get(&o.sco_file)
            .and_then(Option::as_ref)
            .is_some_and(|s| s.has_absolute_height());
        let ground = if absolute {
            0.0
        } else {
            self.terrain.map_or(0.0, |t| {
                opts_bevy::sample_terrain(t, o.position.x, o.position.y)
            })
        };
        f64::from(ground + o.position.z)
    }
}

/// Road lane segments of a tile in the world frame.
pub(super) fn tile_segments(
    job: &mut WorldLoadJob,
    tile: &MapTile,
    terrain: Option<&MapTerrain>,
    origin: (f32, f32),
) -> Vec<LaneSegment> {
    let mut source = JobSource {
        job,
        terrain,
        sli: HashMap::new(),
        sco: HashMap::new(),
    };
    let mut segs = segments_from_tile(
        tile,
        DVec2::new(f64::from(origin.0), f64::from(origin.1)),
        &mut source,
    );
    segs.retain(|s| s.kind == LaneKind::Road);
    segs
}

/// Registers a tile's lanes.
pub(super) fn register_tile(commands: &mut Commands, key: (i32, i32), segments: Vec<LaneSegment>) {
    commands.queue(move |world: &mut World| {
        if let Some(mut state) = world.get_resource_mut::<TrafficState>() {
            state.segments.insert(key, segments);
            state.dirty = true;
        }
    });
}

/// Removes an unloaded tile's lanes.
pub(super) fn unregister_tile(commands: &mut Commands, key: (i32, i32)) {
    commands.queue(move |world: &mut World| {
        if let Some(mut state) = world.get_resource_mut::<TrafficState>() {
            state.segments.remove(&key);
            state.dirty = true;
        }
    });
}

/// Reads `--traffic` and resets the per-session state when a session starts.
pub fn configure_traffic(
    args: Option<Res<crate::AppArgs>>,
    mut settings: ResMut<TrafficSettings>,
    mut state: ResMut<TrafficState>,
    mut pool: ResMut<TrafficPool>,
) {
    settings.max = args
        .and_then(|a| a.0.traffic)
        .unwrap_or(DEFAULT_MAX_TRAFFIC);
    *state = TrafficState::default();
    *pool = TrafficPool::default();
}

/// Rebuilds the lane graph after tiles were loaded or unloaded; agents on lanes that no longer
/// exist are removed (their entities follow in [`move_traffic`]).
pub fn update_traffic_graph(mut state: ResMut<TrafficState>) {
    if !state.dirty {
        return;
    }
    state.dirty = false;
    let mut tiles: Vec<_> = state.segments.keys().copied().collect();
    tiles.sort_unstable();
    let all: Vec<LaneSegment> = tiles
        .iter()
        .flat_map(|k| state.segments[k].iter().cloned())
        .collect();
    let t = std::time::Instant::now();
    let graph = LaneGraph::build(&all, &ConnectParams::default());
    state.road_length_m = graph
        .lane_ids()
        .filter_map(|l| graph.lane(l))
        .map(|l| l.length())
        .sum();
    let (lanes, length_km) = (graph.len(), state.road_length_m / 1000.0);
    let removed = state.world.replace_graph(graph);
    tracing::debug!(
        lanes,
        length_km,
        removed = removed.len(),
        ms = t.elapsed().as_millis() as u64,
        "traffic lane graph rebuilt"
    );
}

fn load_variant(job: &WorldLoadJob, rel: &str) -> Result<Variant, String> {
    let path = job.install.find_file(rel).ok_or("file not found")?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let manifest = BusManifest::parse_bytes(&bytes).map_err(|e| e.to_string())?;
    let dir = path
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();
    let cfg_rel = manifest.model_cfg.clone().ok_or("no [model] entry")?;
    let cfg = job
        .install
        .find_in(&dir, &cfg_rel)
        .ok_or_else(|| format!("model config not found: {cfg_rel}"))?;
    let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let model_dir = cfg
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();
    let delta_height = scan_blocks(&decode_bytes(&bytes))
        .iter()
        .find(|b| b.keyword == "ai_deltaheight")
        .and_then(|b| b.parameters.first())
        .and_then(|p| p.trim().replace(',', ".").parse::<f32>().ok())
        .unwrap_or(0.0);
    let tex_dirs = vec![
        job.install
            .find_in(&dir, "texture")
            .unwrap_or_else(|| dir.join("texture")),
        model_dir.join("texture"),
    ];
    Ok(Variant {
        name: path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        model,
        model_dir,
        tex_dirs,
        delta_height,
        size: None,
    })
}

/// Reads the map's AI list once and parses one vehicle type every few frames.
pub fn load_traffic_variants(
    job: Option<ResMut<WorldLoadJob>>,
    settings: Res<TrafficSettings>,
    mut pool: ResMut<TrafficPool>,
) {
    let Some(job) = job else { return };
    if settings.max == 0 {
        return;
    }
    if !pool.started {
        pool.started = true;
        pool.rng = Some(Rng::new(0xA11_CA45));
        let lists = job
            .install
            .find_in(&job.config.map_dir, "ailists.cfg")
            .and_then(|p| std::fs::read(p).ok())
            .map(|b| AiLists::parse(&b))
            .unwrap_or_default();
        let group = lists
            .group("NormalCars")
            .or_else(|| lists.groups.first())
            .cloned();
        match group {
            Some(g) => {
                tracing::info!(group = %g.name, vehicles = g.entries.len(), "AI traffic group chosen");
                pool.density = job
                    .install
                    .find_in(&job.config.map_dir, "unsched_trafficdens.txt")
                    .and_then(|p| std::fs::read(p).ok())
                    .map(|b| parse_unsched_trafficdens(&b))
                    .and_then(|d| d.into_iter().find(|x| x.name.eq_ignore_ascii_case(&g.name)));
                pool.slots = g
                    .entries
                    .into_iter()
                    .filter(|e| {
                        let f = e.vehicle.to_ascii_lowercase();
                        f.ends_with(".bus") || f.ends_with(".ovh")
                    })
                    .map(|e| Slot {
                        file: e.vehicle,
                        weight: e.weight.unwrap_or(1.0).max(0.0),
                        fails: 0,
                        state: SlotState::Pending,
                    })
                    .collect();
            }
            None => {
                tracing::error!(map = %job.config.map_dir.display(), "no AI group in ailists.cfg; no traffic");
            }
        }
    }
    pool.frames += 1;
    if !pool.frames.is_multiple_of(10) {
        return;
    }
    let Some(i) = pool
        .slots
        .iter()
        .position(|s| matches!(s.state, SlotState::Pending))
    else {
        return;
    };
    let file = pool.slots[i].file.clone();
    match load_variant(&job, &file) {
        Ok(v) => pool.slots[i].state = SlotState::Ready(Box::new(v)),
        Err(e) => {
            tracing::debug!(vehicle = %file, "AI vehicle skipped: {e}");
            pool.slots[i].state = SlotState::Failed;
        }
    }
}

/// Spawns vehicles on lanes around the camera up to the density-derived target.
#[allow(clippy::too_many_arguments)]
pub fn spawn_traffic(
    time: Res<Time>,
    mut elapsed: Local<f32>,
    mut commands: Commands,
    job: Option<ResMut<WorldLoadJob>>,
    mut state: ResMut<TrafficState>,
    mut pool: ResMut<TrafficPool>,
    settings: Res<TrafficSettings>,
    session: Option<Res<InGameSessionInfo>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
) {
    *elapsed += time.delta_secs();
    if *elapsed < SPAWN_INTERVAL_S || settings.max == 0 {
        return;
    }
    *elapsed = 0.0;
    let (Some(mut job), Ok(cam)) = (job, camera.single()) else {
        return;
    };
    // Target count: road length × the group's density at the session's time of day.
    let hour = session.map_or(12.0, |s| {
        s.start_hour as f32 + s.start_minute as f32 / 60.0 + s.sim_elapsed_seconds / 3600.0
    });
    let factor = f64::from(
        pool.density
            .as_ref()
            .map_or(1.0, |d| d.density(day::WEEKDAY, hour))
            .clamp(0.0, 2.0),
    );
    let target =
        ((state.road_length_m / METRES_OF_ROAD_PER_VEHICLE * factor) as usize).min(settings.max);
    if state.world.agents().len() >= target {
        return;
    }
    let ready: Vec<usize> = pool
        .slots
        .iter()
        .enumerate()
        .filter(|(_, s)| matches!(s.state, SlotState::Ready(_)) && s.weight > 0.0)
        .map(|(i, _)| i)
        .collect();
    if ready.is_empty() {
        return;
    }
    let mut rng = pool.rng.take().unwrap_or_else(|| Rng::new(1));
    let result = (|| {
        let centre = DVec2::new(
            f64::from(cam.translation().x),
            -f64::from(cam.translation().z),
        );
        let fwd = cam.forward();
        let view = DVec2::new(f64::from(fwd.x), -f64::from(fwd.z)).normalize_or_zero();
        let (lane_id, s) =
            state
                .world
                .graph()
                .random_spot(centre, SPAWN_MIN_M, SPAWN_MAX_M, &mut rng, |l| {
                    l.kind == LaneKind::Road && l.length() > 6.0 && l.rules.density != Some(0.0)
                })?;
        let lane = state.world.graph().lane(lane_id)?;
        // A density rule below 1 thins out the lane (`0` was excluded above).
        if let Some(d) = lane.rules.density {
            if rng.next_f64() > d.min(1.0) {
                return None;
            }
        }
        let at = lane.geometry.point_at(s);
        // Not in plain view of the camera.
        let to = (at - centre).normalize_or_zero();
        if view.dot(to) > 0.5 && at.distance(centre) < 160.0 {
            return None;
        }
        let limit = lane.speed_limit;
        let total: f32 = ready.iter().map(|&i| pool.slots[i].weight).sum();
        let mut pick = rng.next_f64() as f32 * total;
        let mut slot = ready[0];
        for &i in &ready {
            if pick < pool.slots[i].weight {
                slot = i;
                break;
            }
            pick -= pool.slots[i].weight;
        }
        Some((lane_id, s, limit, slot, at))
    })();
    let Some((lane_id, s, limit, slot, at)) = result else {
        pool.rng = Some(rng);
        return;
    };

    // Build the model (parts are cached, so repeated types are cheap).
    let SlotState::Ready(variant) = &mut pool.slots[slot].state else {
        pool.rng = Some(rng);
        return;
    };
    let lod = variant
        .model
        .lods
        .iter()
        .find(|l| l.threshold == 0.0)
        .or_else(|| variant.model.lods.first())
        .cloned();
    let Some(lod) = lod else {
        pool.slots[slot].state = SlotState::Failed;
        pool.rng = Some(rng);
        return;
    };
    let root = commands
        .spawn((
            Transform::from_translation(omsi_to_bevy_pos(at.extend(0.0).as_vec3())),
            Visibility::Hidden,
            Name::new(variant.name.clone()),
            InGameEntity,
        ))
        .id();
    let (count, _, min, max) = load_segment_meshes(
        &mut job,
        &mut commands,
        &mut meshes,
        &mut materials,
        &mut images,
        root,
        &lod,
        &variant.model_dir.clone(),
        &variant.tex_dirs.clone(),
    );
    if count == 0 {
        // Meshes still decoding in the background, or unreadable: try again a few times.
        commands.entity(root).despawn();
        tracing::debug!(vehicle = %variant.name, "AI vehicle meshes not ready");
        pool.slots[slot].fails += 1;
        if pool.slots[slot].fails >= MAX_LOAD_TRIES {
            pool.slots[slot].state = SlotState::Failed;
        }
        pool.rng = Some(rng);
        return;
    }
    let SlotState::Ready(variant) = &mut pool.slots[slot].state else {
        return;
    };
    let size = *variant.size.get_or_insert_with(|| {
        let len = max.z - min.z;
        if min.z.is_finite() && (2.0..25.0).contains(&len) {
            (len, (-min.z).clamp(0.0, len + 2.0))
        } else {
            (4.4, 2.2)
        }
    });
    let heavy = size.0 > 8.0;
    let mut params = if heavy {
        IdmParams::heavy()
    } else {
        IdmParams::default()
    };
    params.length = f64::from(size.0);
    // Drivers differ a little; the lane's speed limit caps the desired speed.
    params.desired_speed *= 0.9 + 0.2 * rng.next_f64();
    if let Some(l) = limit {
        params.desired_speed = params.desired_speed.min(l);
    }
    let speed = params.desired_speed * 0.8;
    let delta_height = variant.delta_height;
    let name = variant.name.clone();
    pool.rng = Some(rng);
    match state.world.spawn(lane_id, s, speed, params) {
        Ok(id) => {
            tracing::debug!(agent = id.0, vehicle = %name, speed, "AI vehicle spawned");
            commands.entity(root).insert(TrafficVehicle {
                id,
                front_offset: size.1,
                delta_height,
            });
        }
        Err(_) => commands.entity(root).despawn(),
    }
}

/// Steps the agents, places the vehicle models and removes finished or distant ones.
pub fn move_traffic(
    time: Res<Time>,
    mut commands: Commands,
    mut state: ResMut<TrafficState>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut vehicles: Query<(Entity, &TrafficVehicle, &mut Transform, &mut Visibility)>,
) {
    let dt = f64::from(time.delta_secs().min(0.2));
    // Sub-steps of at most 0.1 s keep the car-following law stable.
    let steps = (dt / 0.1).ceil().max(1.0);
    for _ in 0..steps as usize {
        state.world.step(dt / steps);
    }
    state.world.drain_events();
    let cam = camera.single().ok().map(GlobalTransform::translation);
    let mut far = Vec::new();
    for (entity, v, mut transform, mut visibility) in &mut vehicles {
        let graph = state.world.graph();
        let pose = state.world.agent(v.id).and_then(|a| {
            let (p, h) = a.pose(graph)?;
            Some((p, h, a.height(graph)?))
        });
        let Some((front, heading, z)) = pose else {
            commands.entity(entity).despawn();
            continue;
        };
        // The model origin sits `front_offset` behind the front bumper.
        let dir = DVec2::new(heading.sin(), heading.cos());
        let origin = front - dir * f64::from(v.front_offset);
        let bevy = omsi_to_bevy_pos(Vec3::new(
            origin.x as f32,
            origin.y as f32,
            z as f32 + v.delta_height,
        ));
        if cam.is_some_and(|c| c.distance(bevy) > DESPAWN_M) {
            far.push(v.id);
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation = bevy;
        transform.rotation = omsi_euler_to_bevy_quat(-(heading.to_degrees() as f32), 0.0, 0.0);
        if *visibility != Visibility::Inherited {
            *visibility = Visibility::Inherited;
        }
    }
    for id in far {
        state.world.despawn(id);
    }
}
