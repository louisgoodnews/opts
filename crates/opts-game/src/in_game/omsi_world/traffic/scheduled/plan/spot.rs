//! Where scheduled vehicles stand: the planning context, stop objects and spawn places.

use super::*;

/// A stop object of the map: where a bus stops (OMSI frame) and which way it faces.
pub(super) struct StopObject {
    pub(super) pos: DVec2,
    pub(super) heading: f64,
}

/// What one planning pass knows about the world.
pub(super) struct PlanCtx<'a> {
    pub(super) now: f64,
    pub(super) centre: DVec2,
    pub(super) view: DVec2,
    pub(super) graph: &'a opts_traffic::LaneGraph,
    pub(super) stops: HashMap<u32, StopObject>,
    pub(super) tt: &'a opts_core::formats::timetable::Timetables,
}

impl PlanCtx<'_> {
    /// The lane and distance along it where the stop object of `station` stands.
    pub(super) fn place(&self, station: u32) -> Option<(LaneId, f64)> {
        let o = self.stops.get(&station)?;
        self.graph
            .nearest_place(o.pos, o.heading, LANE_HEADING_TOL, LANE_REACH_M, |l| {
                l.kind == LaneKind::Road && l.length() > 4.0
            })
            .map(|(lane, s, _)| (lane, s))
    }
}

/// Where a run is now: the stop it starts at and, between stops, how far it has come; `None` for
/// a run that is not about to start or already over.
pub(super) fn run_start(
    ctx: &PlanCtx,
    schedule: &opts_core::formats::timetable::ActiveSchedule,
) -> Option<(usize, Option<f64>)> {
    let phase = run_phase(schedule, ctx.now);
    Some(match phase {
        RunPhase::NotStarted { wait_s } if wait_s <= LEAD_S => (0, None),
        RunPhase::AtStop { index } => (index, None),
        RunPhase::Between { from, fraction } => (from, Some(fraction)),
        _ => return None,
    })
}

/// Where the vehicle is placed: at the stop, or on the way to the next one; when the stop it
/// comes from is not on loaded lanes but the next one is, it comes from upstream of that.
pub(super) fn spawn_spot(
    ctx: &PlanCtx,
    schedule: &opts_core::formats::timetable::ActiveSchedule,
    (first_stop, offset): (usize, Option<f64>),
) -> Option<(LaneId, f64, Vec<LaneId>)> {
    let (graph, now) = (ctx.graph, ctx.now);
    let a = schedule
        .stops
        .get(first_stop)
        .and_then(|s| ctx.place(s.station_id));
    Some(match (offset, a) {
        (None, Some(a)) => (a.0, a.1, vec![]),
        (None, None) => return None,
        (Some(fraction), a) => {
            let b = schedule
                .stops
                .get(first_stop + 1)
                .and_then(|s| ctx.place(s.station_id))?;
            match a {
                Some(a) => {
                    let (lanes, lengths) = leg(graph, a, b)?;
                    let total: f64 = lengths.iter().sum::<f64>()
                        - a.1
                        - (lengths.last().copied().unwrap_or(0.0) - b.1);
                    let (i, s) = place_along(&lengths, a.1, fraction * total.max(0.0))?;
                    (lanes[i], s, Vec::new())
                }
                None => {
                    let due = schedule.stops[first_stop + 1].arrival_s;
                    let back = ((f64::from(due) - now).max(0.0) * UPSTREAM_MS)
                        .clamp(UPSTREAM_MIN_M, APPEAR_MAX_M);
                    let (lane, s) = walk_back(graph, b.0, b.1, back)?;
                    (lane, s, Vec::new())
                }
            }
        }
    })
}

/// Whether a place is in the ring around the camera where vehicles appear, and not in plain view.
pub(super) fn appears_there(ctx: &PlanCtx, (spawn_lane, spawn_s): (LaneId, f64)) -> bool {
    let Some(lane) = ctx.graph.lane(spawn_lane) else {
        return false;
    };
    let (centre, view) = (ctx.centre, ctx.view);
    let at = lane.geometry.point_at(spawn_s);
    let distance = at.distance(centre);
    let to = (at - centre).normalize_or_zero();
    if !(APPEAR_MIN_M..=APPEAR_MAX_M).contains(&distance)
        || (view.dot(to) > 0.5 && distance < VIEW_CLEAR_M)
    {
        return false;
    }
    true
}
