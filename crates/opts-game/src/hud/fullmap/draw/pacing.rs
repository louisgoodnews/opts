//! When the full map is drawn again: at once on a change, otherwise at a rate that follows the motion.

use super::super::*;

/// Whether the map is drawn now: at once after a resize or a change of the view or the markers
/// while something moves (at most ten times a second), otherwise twice a second.
pub(super) fn redraw_due(
    state: &FullMapState,
    generation: u64,
    resized: bool,
    markers: &[(DVec2, f32, bool)],
) -> bool {
    let view_changed = state.drawn_generation != generation
        || state
            .drawn
            .as_ref()
            .is_none_or(|(v, _)| *v != state.viewport);
    let markers_moved = state.drawn.as_ref().is_none_or(|(_, old)| {
        old.len() != markers.len()
            || old
                .iter()
                .zip(markers)
                .any(|(a, b)| a.0.distance(b.0) > 0.05 || (a.1 - b.1).abs() > 0.01 || a.2 != b.2)
    });

    resized
        || state.immediate
        || (view_changed && state.since >= MOVING_INTERVAL_S)
        || (markers_moved && state.since >= MOVING_INTERVAL_S)
        || state.since >= IDLE_INTERVAL_S
}

/// What the state remembers of this redraw.
pub(super) fn record_redraw(
    state: &mut FullMapState,
    started: std::time::Instant,
    (lines, drawn): (usize, usize),
    (viewport, markers, generation): (Viewport, Vec<(DVec2, f32, bool)>, u64),
) {
    let us = started.elapsed().as_micros() as u64;
    state.frames += 1;
    state.last_us = us;
    state.worst_us = state.worst_us.max(us);
    state.lines = lines;
    state.markers = drawn;
    state.drawn = Some((viewport, markers));
    state.drawn_generation = generation;
}
