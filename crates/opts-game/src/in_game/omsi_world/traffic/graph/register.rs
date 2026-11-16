//! Registration of a tile's lanes, parked vehicles and their removal, sent to the world's
//! traffic state through deferred commands.

use super::super::*;

/// Registers a tile's lanes.
pub(in crate::in_game) fn register_tile(
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

/// Registers a parked vehicle's footprint on tile `key` (session frame, compass heading in
/// radians).
pub(in crate::in_game) fn register_parked(
    commands: &mut Commands,
    key: (i32, i32),
    centre: DVec2,
    heading: f64,
    length: f64,
    width: f64,
) {
    commands.queue(move |world: &mut World| {
        if let Some(mut state) = world.get_resource_mut::<TrafficState>() {
            state
                .parked
                .entry(key)
                .or_default()
                .push(opts_traffic::Obstacle {
                    centre,
                    heading,
                    length,
                    width,
                    velocity: DVec2::ZERO,
                });
        }
    });
}

/// Registers the footprint of a parked vehicle object (`footprint` is its `.sco` bounding box,
/// a car's size when the file has none) on tile `key`.
pub(in crate::in_game) fn register_parked_object(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    key: (i32, i32),
    origin: (f32, f32),
    obj: &MapObjectInstance,
) {
    // The traffic must see the parked vehicle as an obstacle on the lane it stands on.
    let footprint = job
        .sco(&obj.sco_file)
        .and_then(|(_, s)| s.bounding_box.clone());
    let footprint = footprint.as_ref();
    let (length, width, offset) = footprint.map_or((4.3, 1.8, (0.0, 0.0)), |b| {
        (
            f64::from(b.half_extents.y) * 2.0,
            f64::from(b.half_extents.x) * 2.0,
            (f64::from(b.center.x), f64::from(b.center.y)),
        )
    });
    let h = f64::from(obj.heading_deg).to_radians();
    let (fwd, right) = (DVec2::new(h.sin(), h.cos()), DVec2::new(h.cos(), -h.sin()));
    let pos = DVec2::new(
        f64::from(origin.0 + obj.position.x),
        f64::from(origin.1 + obj.position.y),
    );
    register_parked(
        commands,
        key,
        pos + right * offset.0 + fwd * offset.1,
        h,
        length.clamp(2.0, 12.0),
        width.clamp(1.0, 3.0),
    );
}

/// Removes an unloaded tile's lanes.
pub(in crate::in_game) fn unregister_tile(commands: &mut Commands, key: (i32, i32)) {
    commands.queue(move |world: &mut World| {
        if let Some(mut state) = world.get_resource_mut::<TrafficState>() {
            state.segments.remove(&key);
            state.signals.remove(&key);
            state.parked.remove(&key);
            state.dirty = true;
        }
    });
}

/// Empties the pool so that the AI lists are read again (a Chrono switch changed them). Vehicles
/// already driving stay.
pub(in crate::in_game) fn reset_pool(pool: &mut TrafficPool) {
    *pool = TrafficPool::default();
}
