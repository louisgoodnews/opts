//! The view of a tick: the radius from the zoom mode and the speed, and what points up.

use super::super::*;

/// The view for this tick: the radius (the zoom step, or what the speed asks for: the head unit's
/// road speed, the free camera's own speed without a vehicle) and what points up. Records the
/// radius and speed in the state.
pub(super) fn choose_view(
    state: &mut MiniMapState,
    (centre, heading): (DVec2, f32),
    has_player: bool,
    hud: Option<&crate::hud::HudValues>,
    tick_dt: f32,
) -> MapView {
    let config = state.config;
    // The radius: the zoom step, or what the speed asks for (the head unit's road speed; the
    // free camera's own speed when there is no vehicle).
    let moved = state.centre.map(|prev| (centre - prev).length());
    let speed_in = if has_player {
        hud.map(|h| h.speed_kmh)
    } else {
        moved.and_then(|m| zoom::camera_speed_kmh(m, tick_dt))
    };
    if moved.is_some_and(|m| m > zoom::JUMP_M) {
        state.filter.reset();
    }
    let (radius_m, target_m) = match config.zoom_mode {
        ZoomMode::Fixed => {
            state.filter.reset();
            (config.zoom_m, config.zoom_m)
        }
        ZoomMode::Speed => {
            let held = state.filter.speed_kmh;
            let r = state
                .filter
                .step(tick_dt, speed_in.unwrap_or(held), config.zoom_m);
            (
                r,
                zoom::target_radius(state.filter.speed_kmh, config.zoom_m),
            )
        }
    };
    state.radius_m = radius_m;
    state.target_m = target_m;
    state.speed_kmh = state.filter.speed_kmh;
    MapView {
        radius_m,
        up_heading: match config.orientation {
            Orientation::North => 0.0,
            Orientation::Heading => heading,
        },
    }
}
