//! The path source of the lanes: reads a tile's splines, scenery and terrain through the load job's
//! caches, and the lanes and signals of a tile in the world frame.

use super::super::*;

/// Path data for [`segments_from_tile`], read through the load job's caches.
pub(in super::super) struct JobSource<'a> {
    pub(in super::super) job: &'a mut WorldLoadJob,
    pub(in super::super) terrain: Option<&'a TileTerrain>,
    pub(in super::super) sli: HashMap<String, Option<Arc<SplineProfile>>>,
    pub(in super::super) sco: HashMap<String, Option<Arc<SceneryObject>>>,
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
pub(in crate::in_game) fn tile_segments(
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
