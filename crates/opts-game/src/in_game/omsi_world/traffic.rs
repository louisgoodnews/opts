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
use opts_core::formats::map::lists::day;
use opts_core::formats::map::{MapObjectInstance, MapTerrain, MapTile};
use opts_core::formats::scenery::sco::{ScoPath, ScoTrafficLightGroup};
use opts_core::formats::scenery::{SceneryObject, SplinePath, SplineProfile};
use opts_core::formats::vehicles::model::ModelLod;
use opts_traffic::{
    segments_from_tile, signals_from_tile, AgentId, Calibration, ConnectParams, IdmParams,
    LaneGraph, LaneKind, LaneSegment, PathSource, Rng, SignalKey, SignalProgram, TrafficWorld,
    VehicleClass,
};

use crate::in_game::types::InGameEntity;

use super::traffic_chain::{despawn_chain, ChainUnit, TrafficChain};
use super::traffic_variant::{load_variant, read_groups, PoolGroup, Slot, SlotState, Unit};
use super::vehicle::load_segment_meshes_for;
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
    /// Wheel radius (m) and wheelbase (m) used for the wheel variables (from the vehicle's
    /// axles when the file gives them).
    wheel_radius: f32,
    wheelbase: f32,
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
            front_offset: 0.0,
            delta_height: 0.0,
            wheel_radius: 0.31,
            wheelbase: 2.6,
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
            self.terrain.map_or(0.0, |t| {
                opts_bevy::sample_terrain(t, o.position.x, o.position.y)
            })
        };
        f64::from(ground + o.position.z)
    }
}

/// Road lane segments and light programs of a tile in the world frame.
pub(super) fn tile_segments(
    job: &mut WorldLoadJob,
    tile: &MapTile,
    terrain: Option<&MapTerrain>,
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
        let (groups, slots) = read_groups(&job);
        pool.groups = groups;
        pool.slots = slots;
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

/// Picks the group by its density at `hour`, then a vehicle of it by weight.
fn pick_slot(pool: &TrafficPool, factors: &[f64], rng: &mut Rng) -> Option<usize> {
    let ready = |s: &Slot| matches!(s.state, SlotState::Ready(_)) && s.weight > 0.0;
    let live: Vec<usize> = (0..pool.groups.len())
        .filter(|&g| factors[g] > 0.0 && pool.slots.iter().any(|s| s.group == g && ready(s)))
        .collect();
    let total: f64 = live.iter().map(|&g| factors[g]).sum();
    let mut pick = rng.next_f64() * total;
    let mut group = *live.first()?;
    for &g in &live {
        if pick < factors[g] {
            group = g;
            break;
        }
        pick -= factors[g];
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
    // Target count: road length × the summed density of the groups at the session's time of day.
    // The hour wraps at midnight (0..24) and the day of the week picks the curves.
    let (hour, day_bit) = env.map_or((12.0, day::WEEKDAY), |e| (e.hour(), e.day_bit()));
    let factors: Vec<f64> = pool
        .groups
        .iter()
        .map(|g| {
            f64::from(
                g.density
                    .as_ref()
                    .map_or(1.0, |d| d.density(day_bit, hour))
                    .clamp(0.0, 2.0),
            )
        })
        .collect();
    let target = ((state.road_length_m / METRES_OF_ROAD_PER_VEHICLE * factors.iter().sum::<f64>())
        as usize)
        .min(settings.max);
    // A line every 15 s says why no vehicle appears (no road, no density, vehicles not loaded).
    *since_log += SPAWN_INTERVAL_S;
    if *since_log >= 15.0 {
        *since_log = 0.0;
        let count = |f: fn(&SlotState) -> bool| pool.slots.iter().filter(|s| f(&s.state)).count();
        tracing::debug!(
            target,
            agents = state.world.agents().len(),
            road_km = state.road_length_m / 1000.0,
            density = ?factors,
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
    // Why an attempt found no place, counted for the diagnostic line: no vehicle ready, no lane
    // in the ring around the camera, lane gone, thinned by a density rule, in the camera's view.
    let result = (|| -> Result<_, usize> {
        let centre = DVec2::new(
            f64::from(cam.translation().x),
            -f64::from(cam.translation().z),
        );
        let fwd = cam.forward();
        let view = DVec2::new(f64::from(fwd.x), -f64::from(fwd.z)).normalize_or_zero();
        // The vehicle type comes first: bus and truck lanes admit only their own.
        let slot = pick_slot(&pool, &factors, &mut rng).ok_or(0_usize)?;
        let class = match &pool.slots[slot].state {
            SlotState::Ready(v) => v.class(),
            _ => VehicleClass::Car,
        };
        let (lane_id, s) = state
            .world
            .graph()
            .random_spot(centre, SPAWN_MIN_M, SPAWN_MAX_M, &mut rng, |l| {
                l.kind == LaneKind::Road
                    && l.length() > 6.0
                    && l.rules.density != Some(0.0)
                    && l.rules.allows(class)
            })
            .ok_or(1_usize)?;
        let lane = state.world.graph().lane(lane_id).ok_or(2_usize)?;
        // A density rule below 1 thins out the lane (`0` was excluded above).
        if let Some(d) = lane.rules.density {
            if rng.next_f64() > d.min(1.0) {
                return Err(3);
            }
        }
        let at = lane.geometry.point_at(s);
        // Not in plain view of the camera.
        let to = (at - centre).normalize_or_zero();
        if view.dot(to) > 0.5 && at.distance(centre) < 160.0 {
            return Err(4);
        }
        Ok((lane_id, s, lane.speed_limit, slot, at))
    })();
    let (lane_id, s, limit, slot, at) = match result {
        Ok(found) => found,
        Err(reason) => {
            rejected[reason] += 1;
            pool.rng = Some(rng);
            return;
        }
    };

    let group_name = pool.groups[pool.slots[slot].group].name.clone();
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
    let params = IdmParams::from_traits(&traits, &Calibration::default(), rng.next_f64(), limit);
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
    let speed = params.desired_speed * 0.8;
    let head = &variant.units[0];
    let size = head.size.unwrap_or((4.4, 2.2));
    let heavy = params.class != VehicleClass::Car;
    let (wheel_radius, wheelbase) = (
        head.profile
            .wheel_radius_m
            .unwrap_or(if heavy { 0.5 } else { 0.31 }),
        head.profile.wheelbase_m.unwrap_or(size.0 * 0.6),
    );
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
    let mut rng_pax = Rng::new(rng.next_u64());
    pool.rng = Some(rng);
    match state.world.spawn(lane_id, s, speed, params) {
        Ok(id) => {
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
                    front_offset: size.1,
                    delta_height,
                    wheel_radius,
                    wheelbase,
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
    let brightness = clock.map(|c| c.envir_brightness());
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
        // Steering from the lane's curvature under the bicycle model; positive radius = right.
        let steer = state.world.graph().lane(agent.lane).map_or(0.0, |l| {
            let r = l.geometry.radius;
            if l.geometry.is_straight() {
                0.0
            } else {
                (f64::from(v.wheelbase) / r).atan() as f32
            }
        });
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
        let brake = f32::from(command.brake > 0.05);
        vars.set("ai_brakelight", brake);
        vars.set("lights_brems", brake);
        let light = if dark { 1.0 } else { 0.0 };
        vars.set("ai_light", light);
        if let Some(b) = brightness {
            vars.set("envir_brightness", b);
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
    camera: Query<&GlobalTransform, With<Camera3d>>,
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
            let (p, h) = a.pose(graph)?;
            Some((p, h, a.height(graph)?))
        });
        let Some((front, heading, z)) = pose else {
            despawn_chain(&mut commands, entity, chain);
            continue;
        };
        // The model origin sits `front_offset` behind the front bumper.
        let dir = DVec2::new(heading.sin(), heading.cos());
        let origin = front - dir * f64::from(v.front_offset);
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
