//! The mini map control methods: `hud.minimap` (read, or change its settings) and the mini map's
//! state JSON that `hud.state` also reports.

use super::*;

/// The map's settings and what its last redraw did.
pub(super) fn minimap_json(world: &mut World) -> Value {
    let config = world
        .get_resource::<crate::settings::GameSettings>()
        .map(crate::hud::minimap::MiniMapConfig::read)
        .unwrap_or_default();
    let state = world
        .get_resource::<crate::hud::minimap::MiniMapState>()
        .cloned()
        .unwrap_or_default();
    let layout = world
        .get_resource::<crate::hud::layout::HudLayout>()
        .copied()
        .unwrap_or_default();
    let shown = world
        .query_filtered::<&Node, With<crate::hud::parts::MapBox>>()
        .iter(world)
        .next()
        .map(|n| n.display != Display::None);
    json!({
        "enabled": config.enabled,
        "shown": shown.unwrap_or(false),
        "zoom_m": config.zoom_m,
        "zoom_mode": config.zoom_mode.id(),
        "radius_m": state.radius_m,
        "target_m": state.target_m,
        "speed_kmh": state.speed_kmh,
        "orientation": config.orientation.id(),
        "zoom_steps_m": crate::hud::minimap::ZOOM_STEPS_M,
        "texture_px": state.texture_px,
        "side_px": layout.map_side,
        "compact": layout.compact,
        "redraws": state.frames,
        "last_us": state.last_us,
        "worst_us": state.worst_us,
        "budget_us": crate::hud::minimap::BUDGET_US,
        "road_chords": state.lines,
        "markers": state.markers,
        "centre": state.centre.map(|c| json!({ "x": c.x, "y": c.y })),
        "up_heading_deg": state.up_heading_deg,
    })
}

/// `hud.minimap {zoom?, zoom_mode?, orientation?, enabled?}`: reads the mini map, or changes its
/// settings (`hud.minimap`, `hud.minimap_zoom`, `hud.minimap_zoom_mode`, `hud.minimap_orientation`;
/// saved like any setting).
pub(in crate::hud) fn minimap(world: &mut World, params: &Value) -> Outcome {
    use crate::settings_registry::{apply_change, ChangeSource};
    let invalid = |m: &str| Outcome::err(RpcError::new(code::INVALID_PARAMS, m));
    let zoom = match opt_f64(params, "zoom") {
        Ok(z) => z,
        Err(e) => return Outcome::err(e),
    };
    let orientation = match opt_str(params, "orientation") {
        Ok(o) => o,
        Err(e) => return Outcome::err(e),
    };
    let zoom_mode = match opt_str(params, "zoom_mode") {
        Ok(m) => m,
        Err(e) => return Outcome::err(e),
    };
    let enabled = match opt_bool(params, "enabled") {
        Ok(e) => e,
        Err(e) => return Outcome::err(e),
    };
    let mut changes: Vec<(&str, String)> = Vec::new();
    if let Some(z) = zoom {
        if !crate::hud::minimap::ZOOM_STEPS_M
            .iter()
            .any(|s| (f64::from(*s) - z).abs() < 0.5)
        {
            return invalid("zoom is 100, 200, 500 or 1000 (metres of radius)");
        }
        changes.push((
            crate::hud::minimap::SETTING_ZOOM,
            format!("{}", z.round() as i64),
        ));
    }
    if let Some(o) = orientation {
        if crate::hud::minimap::Orientation::parse(o).is_none() {
            return invalid("orientation is north or heading");
        }
        changes.push((crate::hud::minimap::SETTING_ORIENTATION, o.to_owned()));
    }
    if let Some(m) = zoom_mode {
        if crate::hud::minimap::ZoomMode::parse(m).is_none() {
            return invalid("zoom_mode is speed or fixed");
        }
        changes.push((crate::hud::minimap::SETTING_ZOOM_MODE, m.to_owned()));
    }
    if let Some(e) = enabled {
        changes.push((crate::hud::minimap::SETTING_ENABLED, e.to_string()));
    }
    for (key, value) in changes {
        if let Err(e) = apply_change(world, key, Some(&value), ChangeSource::Protocol) {
            return Outcome::err(RpcError::new(code::INVALID_PARAMS, e.to_string()));
        }
    }
    Outcome::ok(minimap_json(world))
}
