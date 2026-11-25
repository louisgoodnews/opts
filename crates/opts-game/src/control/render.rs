//! `render.stats` and `render.look`: frame statistics and the colour look of the renderer.

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_str};
use opts_bevy::control::state::Outcome;
use serde_json::{json, Value};

use opts_cli::protocol::RpcError;

/// `render.stats {reset?, parts?}`: frame times since the last reset, render-phase draw counts,
/// memory, and (with `parts`) distinct meshes/materials of the visible mesh entities.
pub(super) fn render_stats(world: &mut World, params: &Value) -> Outcome {
    let reset = opt_bool(params, "reset").ok().flatten().unwrap_or(false);
    let parts = opt_bool(params, "parts").ok().flatten().unwrap_or(false);
    Outcome::ok(opts_bevy::render_stats::render_stats(world, reset, parts))
}

/// `render.look {look?}`: reads the colour look, or sets it (`vivid`, `natural`, `bevy`). A set
/// takes effect at once on every camera and the sky, and is saved in the settings.
pub(super) fn render_look(world: &mut World, params: &Value) -> Outcome {
    let current = world.resource::<crate::look::ActiveLook>().0;
    let requested = match opt_str(params, "look") {
        Ok(r) => r,
        Err(e) => return Outcome::err(e),
    };
    let look = match requested {
        None => current,
        Some(text) => match crate::look::Look::parse(text) {
            Some(l) => l,
            None => {
                return Outcome::err(RpcError::invalid_params(format!(
                    "unknown look `{text}` (vivid, natural, bevy)"
                )))
            }
        },
    };
    if requested.is_some() {
        world.resource_mut::<crate::look::ActiveLook>().0 = look;
        if let Some(mut settings) = world.get_resource_mut::<crate::settings::GameSettings>() {
            settings.look = look;
            crate::settings::save_in_background(&settings);
        }
    }
    let available: Vec<&str> = crate::look::Look::ALL.iter().map(|l| l.key()).collect();
    Outcome::ok(json!({ "look": look.key(), "previous": current.key(), "available": available }))
}
