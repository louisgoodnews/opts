//! The traffic road graph of the loaded tiles.

use super::*;

/// Path data for [`segments_from_tile`], read through the load job's caches.
pub(super) struct JobSource<'a> {
    pub(super) job: &'a mut WorldLoadJob,
    pub(super) terrain: Option<&'a TileTerrain>,
    pub(super) sli: HashMap<String, Option<Arc<SplineProfile>>>,
    pub(super) sco: HashMap<String, Option<Arc<SceneryObject>>>,
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
pub(in super::super::super) fn tile_segments(
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
pub(in super::super::super) fn register_tile(
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
pub(in super::super::super) fn unregister_tile(commands: &mut Commands, key: (i32, i32)) {
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
pub(in super::super::super) fn reset_pool(pool: &mut TrafficPool) {
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
