//! Control-protocol method `hardware.profile` (`docs/src/reference/control-protocol.md`).

use bevy::prelude::*;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_hardware::report::recommendation_lines;
use serde_json::{json, Value};

use super::HardwareState;
use crate::settings::GameSettings;
use crate::settings_registry::SettingsRegistry;

/// Registers the method when the control server exists.
pub fn register(app: &mut App) {
    let Some(mut registry) = app.world_mut().get_resource_mut::<ControlRegistry>() else {
        return;
    };
    registry.register("hardware.profile", profile);
}

/// The profile document of the method (also used by tests).
pub(crate) fn document(world: &World) -> Value {
    let state = world.resource::<HardwareState>();
    let allow = world
        .get_resource::<SettingsRegistry>()
        .zip(world.get_resource::<GameSettings>())
        .and_then(|(r, s)| r.text(s, super::ALLOW_REPORTS_KEY))
        .is_some_and(|t| t == "true");
    let recommendations: serde_json::Map<String, Value> = state
        .profile
        .as_ref()
        .map(recommendation_lines)
        .unwrap_or_default()
        .into_iter()
        .map(|(k, v)| (k, Value::String(v)))
        .collect();
    json!({
        "probing": state.is_probing(),
        "stored": state.is_stored(),
        "allow_in_reports": allow,
        "profile": state.profile.as_ref().and_then(|p| serde_json::to_value(p).ok()),
        "recommendations": recommendations,
    })
}

fn profile(world: &mut World, params: &Value) -> Outcome {
    if params.get("redetect").and_then(Value::as_bool) == Some(true) {
        world.resource_mut::<HardwareState>().start_probe(false, true);
    }
    Outcome::ok(document(world))
}
