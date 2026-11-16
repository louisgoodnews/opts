//! `vehicle.setvars` with values: validates the batch and applies it, all or none.

use bevy::prelude::*;
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::RpcError;
use opts_core::setvars::SetvarCatalogue;
use serde_json::{json, Map, Value};

use crate::new_game_screen::types::NewGameSetup;

use super::apply::apply_live;
use super::describe::{repaint_of, value_of};
use super::source::{Context, Source};
use super::validate::validate;

pub(super) fn set(
    world: &mut World,
    src: &Source,
    catalogue: &SetvarCatalogue,
    values: &Map<String, Value>,
    allow_unlisted: bool,
) -> Outcome {
    if values.is_empty() {
        return Outcome::err(RpcError::invalid_params(
            "parameter `values` has no options",
        ));
    }
    let batch = match validate(catalogue, values, allow_unlisted) {
        Ok(b) => b,
        Err(bad) => {
            let first = bad[0]["message"].as_str().unwrap_or_default().to_string();
            let text = format!(
                "{} of {} options refused, nothing changed; first: {first}",
                bad.len(),
                values.len()
            );
            return Outcome::err(
                RpcError::invalid_params(text).with_data(json!({ "errors": bad })),
            );
        }
    };
    let mut changed = Vec::new();
    for (name, value) in &batch {
        let previous = catalogue.get(name).map_or(0.0, |e| value_of(src, e));
        let default = catalogue
            .get(name)
            .map_or(0, |e| e.default_for(repaint_of(src)).round() as i32);
        match src.context {
            Context::Wizard => {
                let mut setup = world.resource_mut::<NewGameSetup>();
                if *value == default {
                    setup.custom_setvars.remove(name);
                } else {
                    setup.custom_setvars.insert(name.clone(), *value);
                }
            }
            Context::Drive => apply_live(world, name, *value, default),
        }
        changed.push(json!({ "name": name, "previous": previous, "value": value }));
    }
    Outcome::ok(json!({
        "context": src.context.name(),
        "count": batch.len(),
        "changed": changed,
        "live": src.context == Context::Drive,
    }))
}
