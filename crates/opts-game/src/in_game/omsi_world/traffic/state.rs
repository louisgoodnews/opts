//! The lane network of the resident tiles and the agents on it: the state the traffic systems share.

use super::*;

/// The lane network of the resident tiles and the agents on it.
#[derive(Resource)]
pub struct TrafficState {
    pub(super) world: TrafficWorld,
    /// Road lane segments per resident tile.
    pub(super) segments: HashMap<(i32, i32), Vec<LaneSegment>>,
    /// Light programs of the junction objects per resident tile.
    pub(super) signals: HashMap<(i32, i32), Vec<(SignalKey, SignalProgram)>>,
    /// The segments changed; the graph is rebuilt before the next step.
    pub(super) dirty: bool,
    pub(super) road_length_m: f64,
    /// Footprints of the parked vehicles of each resident tile: the traffic stops behind them or
    /// drives round them like round the player's vehicle.
    pub(super) parked: HashMap<(i32, i32), Vec<opts_traffic::Obstacle>>,
}

impl TrafficState {
    /// The parked vehicles within `radius` metres of `centre` (session frame).
    pub(crate) fn parked_near(
        &self,
        centre: DVec2,
        radius: f64,
    ) -> impl Iterator<Item = opts_traffic::Obstacle> + '_ {
        self.parked
            .values()
            .flatten()
            .filter(move |o| o.centre.distance(centre) <= radius)
            .copied()
    }

    /// Behaviour counters of the traffic world (hard brakes, aborted lane changes).
    #[must_use]
    pub(crate) fn stats(&self) -> opts_traffic::TrafficStats {
        self.world.stats()
    }

    /// Vehicles on the lanes (scheduled ones included).
    #[must_use]
    pub(crate) fn agent_count(&self) -> usize {
        self.world.agents().len()
    }

    /// World position and compass heading of the vehicles `keep` accepts (the maps draw the
    /// scheduled ones).
    pub(crate) fn poses_where(&self, keep: impl Fn(AgentId) -> bool, out: &mut Vec<(DVec2, f32)>) {
        let graph = self.world.graph();
        out.extend(
            self.world
                .agents()
                .iter()
                .filter(|a| keep(a.id))
                .filter_map(|a| a.pose(graph))
                .map(|(p, h)| (p, h as f32)),
        );
    }

    /// Sets the obstacles of the next steps: `obstacles` and the parked vehicles within 400 m of
    /// the camera (the traffic lives within 320 m of it).
    pub(in crate::in_game::omsi_world) fn set_obstacles_with_parked(
        &mut self,
        mut obstacles: Vec<opts_traffic::Obstacle>,
        camera: Option<&GlobalTransform>,
    ) {
        if let Some(cam) = camera {
            let centre = DVec2::new(
                f64::from(cam.translation().x),
                -f64::from(cam.translation().z),
            );
            obstacles.extend(self.parked_near(centre, 400.0));
        }
        self.world.set_obstacles(obstacles);
    }

    /// The agents on the lanes (scheduled ones included), in id order.
    #[must_use]
    pub(crate) fn agents(&self) -> &[opts_traffic::Agent] {
        self.world.agents()
    }

    /// The lane graph of the resident tiles.
    #[must_use]
    pub(crate) fn lane_graph(&self) -> &LaneGraph {
        self.world.graph()
    }

    /// Whether the lanes of tile `key` are in the graph (the tile is resident).
    #[must_use]
    pub(crate) fn has_tile(&self, key: (i32, i32)) -> bool {
        self.segments.contains_key(&key)
    }
}

impl Default for TrafficState {
    fn default() -> Self {
        Self {
            world: TrafficWorld::new(LaneGraph::default(), 0x7AF1_C001),
            segments: HashMap::new(),
            signals: HashMap::new(),
            dirty: false,
            road_length_m: 0.0,
            parked: HashMap::new(),
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
