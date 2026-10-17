//! AI road traffic on the lanes of the loaded tiles (kinematic mode, M-0049 phase C).
//!
//! Lanes come from the type-0 `[path]` records of splines and scenery objects
//! ([`opts_traffic::segments_from_tile`]) and join geometrically; vehicles are drawn from the map's
//! `ailists.cfg` (every group that `unsched_vehgroups.txt` lets take part) and driven by the
//! Intelligent Driver Model with parameters derived from each vehicle's own files
//! ([`opts_traffic::IdmParams::from_traits`]). The vehicle models are placed on the lane (no
//! scripts or physics); coupled vehicles pull their trailing units behind them
//! (`traffic_chain.rs`). Plan and limits: `docs/src/architecture/ai-traffic.md`,
//! `docs/src/architecture/ai-vehicle-config.md`.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_bevy::coords::{omsi_euler_to_bevy_quat, omsi_to_bevy_pos};
use opts_bevy::TileTerrain;
use opts_core::formats::map::lists::day;
use opts_core::formats::map::{DensityProfile, MapObjectInstance, MapTile};
use opts_core::formats::scenery::sco::{ScoPath, ScoTrafficLightGroup};
use opts_core::formats::scenery::{SceneryObject, SplinePath, SplineProfile};
use opts_core::formats::vehicles::model::ModelLod;
use opts_traffic::{
    group_targets, segments_from_tile, signals_from_tile, AgentId, Calibration, ConnectParams,
    IdmParams, LaneGraph, LaneKind, LaneSegment, PathSource, Rng, SignalKey, SignalProgram,
    TrafficWorld, VehicleClass,
};

use crate::in_game::types::InGameEntity;

use super::traffic_chain::{despawn_chain, ChainUnit, TrafficChain};
use super::traffic_variant::{
    load_variant, read_groups, PoolGroup, PoolSetup, Slot, SlotState, Unit,
};
use super::vehicle::load_segment_meshes_for;
use super::WorldLoadJob;

/// Default ceiling on simultaneously driving AI vehicles. It is a performance limit, not the
/// density: the map's density model decides how many vehicles there are, and only when that
/// exceeds the ceiling are all groups scaled down in proportion.
pub const DEFAULT_MAX_TRAFFIC: usize = 100;
/// `L0`: one vehicle per this much lane (m) at density `1` and time factor `1` (the density model,
/// `docs/src/architecture/ai-traffic.md`). Calibrated with the `density_census` example: the
/// median, over 56 entry points of 12 maps at 12:00 on a weekday, of the lane metres × density ×
/// time factor within 320 m divided by the former target (84 m; Berlin-Spandau's main entry 60 m).
pub const METRES_OF_LANE_PER_VEHICLE: f64 = 80.0;
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
    /// Light programs of the junction objects per resident tile.
    signals: HashMap<(i32, i32), Vec<(SignalKey, SignalProgram)>>,
    /// The segments changed; the graph is rebuilt before the next step.
    dirty: bool,
    road_length_m: f64,
}

impl Default for TrafficState {
    fn default() -> Self {
        Self {
            world: TrafficWorld::new(LaneGraph::default(), 0x7AF1_C001),
            segments: HashMap::new(),
            signals: HashMap::new(),
            dirty: false,
            road_length_m: 0.0,
        }
    }
}

/// The mini map reads the road paths of the resident tiles (read-only, no copy of the lane graph).
impl crate::hud::minimap::MapSource for TrafficState {
    fn roads_near(
        &self,
        centre: DVec2,
        radius_m: f64,
        out: &mut Vec<crate::hud::minimap::MapRoad>,
    ) {
        for segment in self.segments.values().flatten() {
            if crate::hud::minimap::may_touch(&segment.geometry, centre, radius_m) {
                out.push(crate::hud::minimap::MapRoad {
                    geometry: segment.geometry,
                    width: segment.width,
                });
            }
        }
    }
}

/// Groups and vehicle types of the map's AI lists, loaded on demand.
#[derive(Resource, Default)]
pub struct TrafficPool {
    started: bool,
    groups: Vec<PoolGroup>,
    slots: Vec<Slot>,
    /// Default density of each `unsched_vehgroups.txt` group.
    defaults: Vec<u32>,
    /// `global.cfg` `[trafficdensity_road]`: the time curve of groups without their own.
    road_curve: DensityProfile,
    /// The pool group of every vehicle this pool spawned.
    agent_groups: HashMap<AgentId, usize>,
    /// The ceiling bound at the last spawn attempt (logged when that changes).
    capped: bool,
    rng: Option<Rng>,
    frames: u32,
    /// Vehicles built so far; with the slot it seeds each vehicle's repaint choice.
    built: u64,
}

/// Component of a driving AI vehicle.
#[derive(Component)]
pub struct TrafficVehicle {
    id: AgentId,
    delta_height: f32,
    /// Wheel radius (m) used for the wheel rotation (from the vehicle's file when it gives one).
    wheel_radius: f32,
    /// Longitudinal position of the rear axle in the model frame (m): the model origin sits
    /// `rear_axle_y` along the body from the agent's rear axle.
    rear_axle_y: f32,
    /// Accumulated wheel rotation (rad).
    wheel_angle: f32,
    /// Seat occupation (`ai_pax1…4`): bit `i` shows passenger `i + 1`.
    pax: u8,
    /// Phase offset of the indicator flash (s), so vehicles do not blink in unison.
    blink_phase: f32,
    /// Pose of the model origin in the OMSI frame, set every frame; trailing units follow it.
    pub(super) origin: DVec2,
    /// Compass heading (radians).
    pub(super) heading: f64,
    /// Height of the model origin including `[ai_deltaheight]`.
    pub(super) height: f32,
}

#[cfg(test)]
impl TrafficVehicle {
    /// A vehicle standing at a pose, for the tests of the systems that follow it.
    pub(super) fn at(origin: DVec2, heading: f64, height: f32) -> Self {
        Self {
            id: AgentId(0),
            delta_height: 0.0,
            wheel_radius: 0.31,
            rear_axle_y: -1.3,
            wheel_angle: 0.0,
            pax: 0,
            blink_phase: 0.0,
            origin,
            heading,
            height,
        }
    }
}

/// Path data for [`segments_from_tile`], read through the load job's caches.
struct JobSource<'a> {
    job: &'a mut WorldLoadJob,
    terrain: Option<&'a TileTerrain>,
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

    fn sco_lights(&mut self, f: &str) -> Option<&[ScoTrafficLightGroup]> {
        let job = &mut *self.job;
        self.sco
            .entry(f.to_owned())
            .or_insert_with(|| job.sco(f).map(|(_, o)| o))
            .as_deref()
            .map(|o| o.traffic_light_groups.as_slice())
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
            self.terrain
                .map_or(0.0, |t| t.height(o.position.x, o.position.y))
        };
        f64::from(ground + o.position.z)
    }
}

/// Road lane segments and light programs of a tile in the world frame.
pub(super) fn tile_segments(
    job: &mut WorldLoadJob,
    tile: &MapTile,
    terrain: Option<&TileTerrain>,
    origin: (f32, f32),
) -> (Vec<LaneSegment>, Vec<(SignalKey, SignalProgram)>) {
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
    let signals = signals_from_tile(tile, &mut source);
    (segs, signals)
}

/// Registers a tile's lanes.
pub(super) fn register_tile(
    commands: &mut Commands,
    key: (i32, i32),
    (segments, signals): (Vec<LaneSegment>, Vec<(SignalKey, SignalProgram)>),
) {
    commands.queue(move |world: &mut World| {
        if let Some(mut state) = world.get_resource_mut::<TrafficState>() {
            state.segments.insert(key, segments);
            state.signals.insert(key, signals);
            state.dirty = true;
        }
    });
}

/// Removes an unloaded tile's lanes.
pub(super) fn unregister_tile(commands: &mut Commands, key: (i32, i32)) {
    commands.queue(move |world: &mut World| {
        if let Some(mut state) = world.get_resource_mut::<TrafficState>() {
            state.segments.remove(&key);
            state.signals.remove(&key);
            state.dirty = true;
        }
    });
}

/// Reads `--traffic` and resets the per-session state when a session starts.
pub fn configure_traffic(
    args: Option<Res<crate::AppArgs>>,
    user: Option<Res<crate::settings::GameSettings>>,
    mut settings: ResMut<TrafficSettings>,
    mut state: ResMut<TrafficState>,
    mut pool: ResMut<TrafficPool>,
) {
    // The command line wins for this run; then the setting *Road traffic*; then the default.
    settings.max = args
        .and_then(|a| a.0.traffic)
        .or_else(|| user.and_then(|u| u.raw("gameplay.traffic")?.parse().ok()))
        .unwrap_or(DEFAULT_MAX_TRAFFIC);
    *state = TrafficState::default();
    *pool = TrafficPool::default();
}

/// Empties the pool so that the AI lists are read again (a Chrono switch changed them). Vehicles
/// already driving stay.
pub(super) fn reset_pool(pool: &mut TrafficPool) {
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
    let programs: HashMap<SignalKey, SignalProgram> = tiles
        .iter()
        .flat_map(|k| state.signals[k].iter().cloned())
        .collect();
    let lights = programs.len();
    state.world.set_signals(programs);
    tracing::debug!(
        lanes,
        length_km,
        lights,
        removed = removed.len(),
        ms = t.elapsed().as_millis() as u64,
        "traffic lane graph rebuilt"
    );
}

/// Reads the map's AI lists once and parses one vehicle type every few frames.
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
        let PoolSetup {
            groups,
            slots,
            defaults,
        } = read_groups(&job);
        pool.groups = groups;
        pool.slots = slots;
        pool.defaults = defaults;
        pool.road_curve = DensityProfile {
            days: 0,
            points: job.manifest.traffic_density_road.clone(),
        };
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

/// The meshes of one unit: the plain `[LOD] 0` body for cars; for buses the full-detail group
/// seen from outside, with the AI-only meshes (`[viewpoint]` 4: interiors of the AI model) and
/// without the cockpit-only ones.
fn unit_lod(unit: &Unit) -> Option<ModelLod> {
    if unit.profile.has_cabin {
        unit.model.detail_lod().map(|mut lod| {
            lod.meshes.retain(|m| {
                let raw = m.viewpoint.raw();
                raw == 0 || raw & 1 != 0 || raw & 4 != 0
            });
            for m in &mut lod.meshes {
                if m.viewpoint.raw() & 4 != 0 {
                    m.viewpoint = opts_core::formats::vehicles::model::ViewpointFlags::ALWAYS;
                }
            }
            lod
        })
    } else {
        unit.model
            .lods
            .iter()
            .find(|l| l.threshold == 0.0)
            .or_else(|| unit.model.lods.first())
            .cloned()
    }
}

/// Time factor of a group: its `unsched_trafficdens.txt` factor times its curve for the hour and
/// day (`[set_day_of_week]`), or the map's `[trafficdensity_road]` curve when the group has no
/// entry there. Unclamped: values above 1 mean denser traffic.
pub(super) fn time_factor(
    group: &PoolGroup,
    road_curve: &DensityProfile,
    day_bit: u8,
    hour: f32,
) -> f64 {
    f64::from(match &group.density {
        Some(d) => d.density(day_bit, hour),
        None => road_curve.at(hour),
    })
    .max(0.0)
}

/// Lane metres inside the disc of `radius` around `centre`, weighted by each pool group's
/// density on the lane: `Σ_lanes length_inside × D(lane, group)`.
pub(super) fn group_lane_metres(
    graph: &LaneGraph,
    centre: DVec2,
    radius: f64,
    groups: &[PoolGroup],
    defaults: &[u32],
) -> Vec<f64> {
    const SAMPLES: usize = 8;
    let mut out = vec![0.0; groups.len()];
    for lane in graph.lane_ids().filter_map(|l| graph.lane(l)) {
        if lane.kind != LaneKind::Road {
            continue;
        }
        let len = lane.length();
        let inside = (0..=SAMPLES)
            .filter(|&i| {
                lane.geometry
                    .point_at(len * i as f64 / SAMPLES as f64)
                    .distance(centre)
                    <= radius
            })
            .count();
        if inside == 0 {
            continue;
        }
        let metres = len * inside as f64 / (SAMPLES + 1) as f64;
        for (g, group) in groups.iter().enumerate() {
            out[g] += metres * lane.rules.density.for_group(group.density_group, defaults);
        }
    }
    out
}

/// Picks a group that is below its target (weighted by how far below) and has a vehicle ready,
/// then a vehicle of it by weight.
fn pick_slot(pool: &TrafficPool, deficits: &[f64], rng: &mut Rng) -> Option<usize> {
    let ready = |s: &Slot| matches!(s.state, SlotState::Ready(_)) && s.weight > 0.0;
    let live: Vec<usize> = (0..pool.groups.len())
        .filter(|&g| deficits[g] > 0.0 && pool.slots.iter().any(|s| s.group == g && ready(s)))
        .collect();
    let total: f64 = live.iter().map(|&g| deficits[g]).sum();
    let mut pick = rng.next_f64() * total;
    let mut group = *live.first()?;
    for &g in &live {
        if pick < deficits[g] {
            group = g;
            break;
        }
        pick -= deficits[g];
    }
    let in_group: Vec<usize> = (0..pool.slots.len())
        .filter(|&i| pool.slots[i].group == group && ready(&pool.slots[i]))
        .collect();
    let weight: f32 = in_group.iter().map(|&i| pool.slots[i].weight).sum();
    let mut pick = rng.next_f64() as f32 * weight;
    let mut slot = *in_group.first()?;
    for &i in &in_group {
        if pick < pool.slots[i].weight {
            slot = i;
            break;
        }
        pick -= pool.slots[i].weight;
    }
    Some(slot)
}

/// Spawns vehicles on lanes around the camera up to the density-derived target.
#[allow(clippy::too_many_arguments)]
pub fn spawn_traffic(
    time: Res<Time>,
    mut elapsed: Local<f32>,
    mut since_log: Local<f32>,
    mut rejected: Local<[u32; 5]>,
    mut commands: Commands,
    job: Option<ResMut<WorldLoadJob>>,
    mut state: ResMut<TrafficState>,
    mut pool: ResMut<TrafficPool>,
    settings: Res<TrafficSettings>,
    env: Option<Res<opts_bevy::SessionEnvironment>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
) {
    *elapsed += time.delta_secs();
    if *elapsed < SPAWN_INTERVAL_S || settings.max == 0 {
        return;
    }
    *elapsed = 0.0;
    let (Some(mut job), Ok(cam)) = (job, camera.single()) else {
        return;
    };
    let centre = DVec2::new(
        f64::from(cam.translation().x),
        -f64::from(cam.translation().z),
    );
    // Density model: per group, the lane metres within the despawn disc weighted by the group's
    // density on each lane, times its time factor for the session's hour and day, per `L0`.
    let (hour, day_bit) = env.map_or((12.0, day::WEEKDAY), |e| (e.hour(), e.day_bit()));
    let factors: Vec<f64> = pool
        .groups
        .iter()
        .map(|g| time_factor(g, &pool.road_curve, day_bit, hour))
        .collect();
    let metres = group_lane_metres(
        state.world.graph(),
        centre,
        f64::from(DESPAWN_M),
        &pool.groups,
        &pool.defaults,
    );
    let wanted: Vec<f64> = metres
        .iter()
        .zip(&factors)
        .map(|(m, f)| m * f / METRES_OF_LANE_PER_VEHICLE)
        .collect();
    let (targets, capped) = group_targets(&wanted, settings.max);
    if capped != pool.capped {
        pool.capped = capped;
        let wanted_total: f64 = wanted.iter().sum();
        if capped {
            tracing::info!(
                wanted = wanted_total,
                ceiling = settings.max,
                scale = settings.max as f64 / wanted_total,
                "traffic ceiling binds: every group scaled down in proportion"
            );
        } else {
            tracing::info!(
                wanted = wanted_total,
                ceiling = settings.max,
                "traffic below the ceiling again"
            );
        }
    }
    let target = (targets.iter().sum::<f64>().round() as usize).min(settings.max);
    // Vehicles per group.
    let TrafficPool {
        agent_groups,
        groups,
        ..
    } = &mut *pool;
    agent_groups.retain(|id, _| state.world.agent(*id).is_some());
    let mut alive = vec![0usize; groups.len()];
    for &g in agent_groups.values() {
        if let Some(a) = alive.get_mut(g) {
            *a += 1;
        }
    }
    let deficits: Vec<f64> = targets
        .iter()
        .zip(&alive)
        .map(|(t, &a)| t - a as f64)
        .collect();
    // A line every 15 s says why no vehicle appears (no road, no density, vehicles not loaded).
    *since_log += SPAWN_INTERVAL_S;
    if *since_log >= 15.0 {
        *since_log = 0.0;
        let count = |f: fn(&SlotState) -> bool| pool.slots.iter().filter(|s| f(&s.state)).count();
        tracing::debug!(
            target,
            agents = state.world.agents().len(),
            road_km = state.road_length_m / 1000.0,
            hour,
            day_bit,
            time_factors = ?factors,
            wanted = ?wanted,
            alive = ?alive,
            capped,
            ready = count(|s| matches!(s, SlotState::Ready(_))),
            pending = count(|s| matches!(s, SlotState::Pending)),
            failed = count(|s| matches!(s, SlotState::Failed)),
            rejected = ?*rejected,
            "traffic state"
        );
    }
    if state.world.agents().len() >= target {
        return;
    }
    let mut rng = pool.rng.take().unwrap_or_else(|| Rng::new(1));
    // Why an attempt found no place, counted for the diagnostic line: no vehicle ready (or no
    // group below its target), no lane in the ring around the camera, lane gone, -, in the
    // camera's view.
    let result = (|| -> Result<_, usize> {
        let fwd = cam.forward();
        let view = DVec2::new(f64::from(fwd.x), -f64::from(fwd.z)).normalize_or_zero();
        // The group and vehicle type come first: bus and truck lanes admit only their own, and
        // the lane is drawn by length × the group's density on it.
        let slot = pick_slot(&pool, &deficits, &mut rng).ok_or(0_usize)?;
        let class = match &pool.slots[slot].state {
            SlotState::Ready(v) => v.class(),
            _ => VehicleClass::Car,
        };
        let group = pool.groups[pool.slots[slot].group].density_group;
        let defaults = &pool.defaults;
        let (lane_id, s) = state
            .world
            .graph()
            .weighted_spot(centre, SPAWN_MIN_M, SPAWN_MAX_M, &mut rng, |l| {
                if l.kind == LaneKind::Road && l.length() > 6.0 && l.rules.allows(class) {
                    l.length() * l.rules.density.for_group(group, defaults)
                } else {
                    0.0
                }
            })
            .ok_or(1_usize)?;
        let lane = state.world.graph().lane(lane_id).ok_or(2_usize)?;
        let at = lane.geometry.point_at(s);
        // Not in plain view of the camera.
        let to = (at - centre).normalize_or_zero();
        if view.dot(to) > 0.5 && at.distance(centre) < 160.0 {
            return Err(4);
        }
        Ok((lane_id, s, slot, at))
    })();
    let (lane_id, s, slot, at) = match result {
        Ok(found) => found,
        Err(reason) => {
            rejected[reason] += 1;
            pool.rng = Some(rng);
            return;
        }
    };

    let pool_group = pool.slots[slot].group;
    let group_name = pool.groups[pool_group].name.clone();
    let density_group = u8::try_from(pool.groups[pool_group].density_group).unwrap_or(u8::MAX);
    pool.built += 1;
    let repaint_seed = crate::new_game_screen::repaints::instance_seed(&[slot as u64, pool.built]);
    // Build the models of every unit (parts are cached, so repeated types are cheap).
    let SlotState::Ready(variant) = &pool.slots[slot].state else {
        pool.rng = Some(rng);
        return;
    };
    let origin = Transform::from_translation(omsi_to_bevy_pos(at.extend(0.0).as_vec3()));
    let entities: Vec<Entity> = variant
        .units
        .iter()
        .map(|u| {
            commands
                .spawn((
                    origin,
                    Visibility::Hidden,
                    Name::new(u.name.clone()),
                    InGameEntity,
                ))
                .id()
        })
        .collect();
    let root = entities[0];
    super::trace::begin_frame();
    let mut bounds = Vec::with_capacity(entities.len());
    let mut failed = None;
    for (unit, &entity) in variant.units.iter().zip(&entities) {
        let Some(lod) = unit_lod(unit) else {
            failed = Some(true);
            break;
        };
        job.vehicle_text_defs = unit.model.text_textures.clone();
        // Each vehicle wears one of its `[CTC]` colour schemes (all units the same seed).
        job.cache.texture_replacements = unit
            .repaints
            .replacements(unit.repaints.pick(repaint_seed), &unit.model.ctc_textures)
            .into_iter()
            .collect();
        let (count, _, min, max) = load_segment_meshes_for(
            &mut job,
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut images,
            entity,
            root,
            &lod,
            &unit.model_dir.clone(),
            &unit.tex_dirs.clone(),
        );
        job.cache.texture_replacements.clear();
        if count == 0 {
            failed = Some(false);
            break;
        }
        bounds.push((min, max));
    }
    let trace_frame = super::trace::end_frame();
    if let Some(unusable) = failed {
        // No usable meshes (unusable), or still decoding in the background: try again a few times.
        let name = variant.name.clone();
        for e in entities {
            commands.entity(e).despawn();
        }
        tracing::debug!(vehicle = %name, "AI vehicle meshes not ready");
        if unusable {
            pool.slots[slot].state = SlotState::Failed;
        } else {
            pool.slots[slot].fails += 1;
            if pool.slots[slot].fails >= MAX_LOAD_TRIES {
                pool.slots[slot].state = SlotState::Failed;
            }
        }
        pool.rng = Some(rng);
        return;
    }
    let SlotState::Ready(variant) = &mut pool.slots[slot].state else {
        return;
    };
    for (unit, (min, max)) in variant.units.iter_mut().zip(&bounds) {
        unit.size.get_or_insert_with(|| {
            let len = max.z - min.z;
            if min.z.is_finite() && (2.0..25.0).contains(&len) {
                (len, (-min.z).clamp(0.0, len + 2.0))
            } else {
                (4.4, 2.2)
            }
        });
    }
    let traits = variant.traits();
    let mut params = IdmParams::from_traits(&traits, &Calibration::default(), rng.next_f64());
    params.density_group = density_group;
    // The vehicle may not use the lane it was placed on (its length was unknown before).
    if state
        .world
        .graph()
        .lane(lane_id)
        .is_some_and(|l| !l.rules.allows(params.class))
    {
        for e in entities {
            commands.entity(e).despawn();
        }
        pool.rng = Some(rng);
        return;
    }
    let speed = state.world.lane_cap(&params, lane_id) * 0.8;
    let head = &variant.units[0];
    let size = head.size.unwrap_or((4.4, 2.2));
    let heavy = params.class != VehicleClass::Car;
    let wheel_radius = head
        .profile
        .wheel_radius_m
        .unwrap_or(if heavy { 0.5 } else { 0.31 });
    // Model frame: front bumper at `size.1`, front axle the overhang behind it, rear axle a
    // wheelbase further back.
    let rear_axle_y = size.1 - (params.front_overhang + params.wheelbase) as f32;
    let delta_height = head.profile.delta_height_m;
    let name = variant.name.clone();
    let fallbacks = head.profile.fallbacks.clone();
    let measured = head.response.is_some();
    let chain: Vec<ChainUnit> = variant
        .units
        .iter()
        .zip(&entities)
        .skip(1)
        .map(|(u, &e)| {
            ChainUnit::new(
                e,
                u.hitch,
                u.joint,
                u.rear_axle_y,
                u.max_alpha_deg,
                u.reversed,
            )
        })
        .collect();
    // Light effects (`[light_enh_2]`: headlights, tail and brake lights, indicators) of every
    // unit, driven by the head's AI variables.
    for (unit, &e) in variant.units.iter().zip(&entities) {
        opts_bevy::spawn_vehicle_glows(&mut commands, root, e, &unit.model);
    }
    let mut rng_pax = Rng::new(rng.next_u64());
    pool.rng = Some(rng);
    match state.world.spawn(lane_id, s, speed, params) {
        Ok(id) => {
            pool.agent_groups.insert(id, pool_group);
            tracing::debug!(
                agent = id.0,
                vehicle = %name,
                group = %group_name,
                speed,
                mass_kg = traits.mass_kg,
                desired_kmh = params.desired_speed * 3.6,
                accel = params.max_accel,
                comfort = params.comfort_decel,
                max_decel = params.max_decel,
                class = ?params.class,
                units = entities.len(),
                measured,
                fallbacks = ?fallbacks,
                "AI vehicle spawned"
            );
            if opts_bevy::scene_trace::enabled() {
                super::trace::spawned(
                    opts_bevy::scene_trace::kind::TRAFFIC,
                    &name,
                    &name,
                    root,
                    &origin,
                    &trace_frame,
                    "ok",
                    serde_json::json!({
                        "lane": format!("{lane_id:?}"),
                        "speed": speed,
                        "class": format!("{:?}", params.class),
                        "units": entities.len(),
                    }),
                );
            }
            // Seats: the driver is nearly always there, further passengers by chance.
            let mut pax = 0u8;
            for (i, p) in [0.9, 0.35, 0.2, 0.1].into_iter().enumerate() {
                if rng_pax.next_f64() < p {
                    pax |= 1 << i;
                }
            }
            commands.entity(root).insert((
                TrafficVehicle {
                    id,
                    delta_height,
                    wheel_radius,
                    rear_axle_y,
                    wheel_angle: 0.0,
                    pax,
                    blink_phase: rng_pax.next_f64() as f32 * 0.7,
                    origin: at,
                    heading: 0.0,
                    height: delta_height,
                },
                opts_bevy::AiVariables::default(),
            ));
            if !chain.is_empty() {
                commands.entity(root).insert(TrafficChain { units: chain });
            }
        }
        Err(_) => {
            for e in entities {
                commands.entity(e).despawn();
            }
        }
    }
}

/// Period of the indicator flash (s); half of it on, half off.
const BLINK_PERIOD_S: f32 = 0.7;

/// Feeds the variables the AI vehicle models read: wheel rotation and steering from speed and lane
/// curvature, indicators, headlights, brake lights and passengers.
///
/// `ai_light` follows the values the AI scripts of OMSI map to their lamps (`0` off, `0.5` position
/// lights, `1` dipped beam, `2` high beam; `docs/src/architecture/ai-traffic.md`, §2.2).
pub fn update_ai_variables(
    time: Res<Time>,
    state: Res<TrafficState>,
    env: Option<Res<opts_bevy::SessionEnvironment>>,
    clock: Option<Res<opts_bevy::SceneryClock>>,
    mut vehicles: Query<(&mut TrafficVehicle, &mut opts_bevy::AiVariables)>,
) {
    let dt = time.delta_secs().min(0.2);
    let brightness = clock.as_ref().map(|c| c.envir_brightness());
    let weather = clock.map(|c| (c.temperature_c, c.weather));
    let now = time.elapsed_secs();
    let dark = env.is_some_and(|e| e.night());
    for (mut v, mut vars) in &mut vehicles {
        let Some(agent) = state.world.agent(v.id) else {
            continue;
        };
        let Some(command) = state.world.command(v.id) else {
            continue;
        };
        v.wheel_angle = (v.wheel_angle + agent.speed as f32 / v.wheel_radius * dt)
            % (std::f32::consts::TAU * 64.0);
        // Steering: the lane's heading at the front axle against the body (positive = right).
        let steer = agent.steer as f32;
        for axle in 0..4 {
            for side in ["l", "r"] {
                vars.set(&format!("wheel_rotation_{axle}_{side}"), v.wheel_angle);
                let s = if axle == 0 { -steer } else { 0.0 };
                vars.set(&format!("axle_steering_{axle}_{side}"), s);
            }
        }
        let flash = (now + v.blink_phase) % BLINK_PERIOD_S < BLINK_PERIOD_S / 2.0;
        let (left, right) = (
            f32::from(flash && command.blinker == opts_traffic::Blinker::Left),
            f32::from(flash && command.blinker == opts_traffic::Blinker::Right),
        );
        vars.set("ai", 1.0);
        vars.set(
            "ai_blinker_l",
            f32::from(command.blinker == opts_traffic::Blinker::Left),
        );
        vars.set(
            "ai_blinker_r",
            f32::from(command.blinker == opts_traffic::Blinker::Right),
        );
        vars.set("lights_blinker_l", left);
        vars.set("lights_blinker_r", right);
        // Brake lights also while standing (held on the brake).
        let brake = f32::from(command.brake > 0.05 || agent.speed < 0.3);
        vars.set("ai_brakelight", brake);
        vars.set("lights_brems", brake);
        let light = if dark { 1.0 } else { 0.0 };
        vars.set("ai_light", light);
        if let Some(b) = brightness {
            vars.set("envir_brightness", b);
        }
        if let Some((temperature, w)) = weather {
            vars.set("weather_temperature", temperature);
            vars.set("weather_abshum", w.abs_hum);
            vars.set("preciptype", w.precip_type);
            vars.set("preciprate", w.precip_rate);
        }
        vars.set("lights_ai", light);
        for i in 0..4 {
            vars.set(&format!("ai_pax{}", i + 1), f32::from(v.pax >> i & 1));
        }
    }
}

/// Steps the agents, places the vehicle models and removes finished or distant ones.
pub fn move_traffic(
    time: Res<Time>,
    mut commands: Commands,
    env: Option<Res<opts_bevy::SessionEnvironment>>,
    mut state: ResMut<TrafficState>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut vehicles: Query<(
        Entity,
        &mut TrafficVehicle,
        &mut Transform,
        &mut Visibility,
        Option<&TrafficChain>,
    )>,
) {
    let dt = f64::from(time.delta_secs().min(0.2));
    // The lights follow the session's time of day.
    if let Some(e) = &env {
        state.world.set_clock(e.seconds_of_day());
    }
    // Sub-steps of at most 0.1 s keep the car-following law stable.
    let steps = (dt / 0.1).ceil().max(1.0);
    for _ in 0..steps as usize {
        state.world.step(dt / steps);
    }
    state.world.drain_events();
    let cam = camera.single().ok().map(GlobalTransform::translation);
    let mut far = Vec::new();
    for (entity, mut v, mut transform, mut visibility, chain) in &mut vehicles {
        let graph = state.world.graph();
        let pose = state.world.agent(v.id).and_then(|a| {
            let (_, h) = a.pose(graph)?;
            Some((a.rear_axle, h, a.height(graph)?))
        });
        let Some((rear, heading, z)) = pose else {
            despawn_chain(&mut commands, entity, chain);
            continue;
        };
        // The body lies along the axles; the model origin sits `rear_axle_y` from the rear axle.
        let dir = DVec2::new(heading.sin(), heading.cos());
        let origin = rear - dir * f64::from(v.rear_axle_y);
        let height = z as f32 + v.delta_height;
        let bevy = omsi_to_bevy_pos(Vec3::new(origin.x as f32, origin.y as f32, height));
        if cam.is_some_and(|c| c.distance(bevy) > DESPAWN_M) {
            far.push(v.id);
            despawn_chain(&mut commands, entity, chain);
            continue;
        }
        (v.origin, v.heading, v.height) = (origin, heading, height);
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

#[cfg(test)]
mod density_tests {
    use super::*;
    use opts_core::formats::map::GroupDensity;

    fn group(name: &str, density_group: usize, density: Option<GroupDensity>) -> PoolGroup {
        PoolGroup {
            name: name.into(),
            density,
            density_group,
        }
    }

    /// Berlin-Spandau's groups: NormalCars 1, Trucks 0, Commercials 1, Ambulance 1, GDRCars 0.
    const SPANDAU: [u32; 5] = [1, 0, 1, 1, 0];

    fn lane(x: f64, rules: &[(usize, f64)]) -> LaneSegment {
        let mut seg = LaneSegment::straight(DVec2::new(x, 0.0), 0.0, 100.0, LaneKind::Road);
        for &(g, v) in rules {
            seg.rules.density.set(g, v);
        }
        seg
    }

    /// GDRCars (default 0) drive only on lanes whose rule has flag 4; values above 1 count fully.
    #[test]
    fn group_density_comes_from_the_rule_flag() {
        let graph = LaneGraph::build(
            &[
                lane(0.0, &[(0, 1.0)]),
                lane(10.0, &[(0, 0.0), (4, 3.0)]),
                lane(20.0, &[]),
                lane(30.0, &[(1, 1.0)]),
            ],
            &ConnectParams::default(),
        );
        let groups = [
            group("NormalCars", 0, None),
            group("Trucks", 1, None),
            group("Commercials", 2, None),
            group("GDRCars", 4, None),
        ];
        let m = group_lane_metres(&graph, DVec2::new(15.0, 50.0), 1000.0, &groups, &SPANDAU);
        // NormalCars: rule 1 + none (Medium 1) + none on the trucks lane (Medium 1); closed by 0.
        assert!((m[0] - 300.0).abs() < 1e-6, "{m:?}");
        assert!(
            (m[1] - 100.0).abs() < 1e-6,
            "Trucks only where a flag-1 rule is: {m:?}"
        );
        assert!(
            (m[2] - m[0]).abs() < 1e-6,
            "Commercials follow NormalCars: {m:?}"
        );
        assert!(
            (m[3] - 300.0).abs() < 1e-6,
            "GDRCars: flag-4 lane at 3x, nowhere else: {m:?}"
        );
    }

    /// Only the part of a lane inside the disc counts.
    #[test]
    fn lane_metres_count_inside_the_disc_only() {
        let graph = LaneGraph::build(&[lane(0.0, &[])], &ConnectParams::default());
        let m = group_lane_metres(
            &graph,
            DVec2::new(0.0, 0.0),
            49.0,
            &[group("N", 0, None)],
            &[1],
        );
        assert!(m[0] > 30.0 && m[0] < 60.0, "{m:?}");
    }

    /// Time factors: the group's own curve (hour wrapping, weekend curve), else the map's road curve.
    #[test]
    fn time_factors_wrap_and_follow_the_day() {
        let own = opts_core::formats::map::parse_unsched_trafficdens(
            b"[group]\nNormalCars\n0.6\n[set_day_of_week]\n1\n[trafficdensity]\n0\n0.1\n[trafficdensity]\n12\n1.5\n[trafficdensity]\n24\n0.1\n[set_day_of_week]\n6\n[trafficdensity]\n0\n0.2\n[trafficdensity]\n24\n0.2\n",
        );
        let g = group("NormalCars", 0, own.into_iter().next());
        let road = DensityProfile {
            days: 0,
            points: vec![(0.0, 0.5), (24.0, 0.5)],
        };
        let f = |day, h| time_factor(&g, &road, day, h);
        assert!(
            (f(day::WEEKDAY, 12.0) - 0.9).abs() < 1e-6,
            "0.6 × 1.5, unclamped curve"
        );
        assert!((f(day::WEEKDAY, 36.0) - f(day::WEEKDAY, 12.0)).abs() < 1e-6);
        assert!((f(day::SUNDAY, 12.0) - 0.12).abs() < 1e-6);
        let none = group("GDRCars", 4, None);
        assert!((time_factor(&none, &road, day::WEEKDAY, 3.0) - 0.5).abs() < 1e-6);
    }

    /// At the ceiling every group keeps its share.
    #[test]
    fn ceiling_scales_groups_in_proportion() {
        let wanted = [90.0, 30.0, 0.3];
        let (t, capped) = group_targets(&wanted, DEFAULT_MAX_TRAFFIC);
        assert!(capped);
        assert!((t.iter().sum::<f64>() - DEFAULT_MAX_TRAFFIC as f64).abs() < 1e-9);
        assert!((t[0] / t[1] - 3.0).abs() < 1e-9);
    }
}
