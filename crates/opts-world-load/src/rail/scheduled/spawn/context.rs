//! The read-only context of one spawn pass: the network, timetables, clock and camera, and the
//! layout of a run's stops on the tracks.

use super::super::*;
use opts_core::formats::timetable::Timetables;
use opts_sim::pedestrian::PedestrianNet;

/// The read-only context of one spawn pass.
pub(in super::super) struct Pass<'a> {
    pub(super) net: &'a PedestrianNet,
    pub(super) tt: &'a Timetables,
    pub(super) now: f64,
    /// Camera position and horizontal view direction (session frame).
    pub(super) centre: Vec3,
    pub(super) view: Vec2,
    /// Stop object positions by station id.
    pub(super) stops: HashMap<u32, Vec2>,
}

/// A run's stop objects on the tracks, with the stop index of each.
pub(in super::super) type Places = Vec<(usize, Place)>;

/// Where a run is and where its train starts: its stops on loaded track, whether the train comes
/// from upstream, the first stop, and the part of the way done (`None` before the start).
pub(in super::super) struct Layout {
    pub(super) places: Places,
    pub(super) upstream: bool,
    pub(super) from_stop: usize,
    pub(super) offset: Option<f64>,
}

/// Where a run's train appears: the run's layout, its route, whether the first place is skipped
/// for stop points, and the lane index and arc length on the route.
pub(in super::super) struct Found {
    pub(super) lay: Layout,
    pub(super) plan: RailRoute,
    pub(super) skip_first: bool,
    pub(super) i: usize,
    pub(super) s: f32,
}

impl<'a> Pass<'a> {
    /// The context from the network, timetables and clock, the camera and the stop positions.
    pub(in super::super) fn new(
        (net, tt, now): (&'a PedestrianNet, &'a Timetables, f64),
        cam: &GlobalTransform,
        stops: HashMap<u32, Vec2>,
    ) -> Self {
        let fwd = cam.forward();
        Self {
            net,
            tt,
            now,
            centre: Vec3::new(cam.translation().x, -cam.translation().z, 0.0),
            view: Vec2::new(fwd.x, -fwd.z).normalize_or_zero(),
            stops,
        }
    }

    /// The place of the stop object of `station` on the tracks.
    pub(super) fn place(&self, station: u32) -> Option<Place> {
        let p = self.stops.get(&station)?;
        let (lane, s, _) = self.net.nearest(Vec3::new(p.x, p.y, 0.0), TRACK_REACH_M)?;
        Some(Place { lane, s })
    }
}
