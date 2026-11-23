//! The exhaust outlet of a vehicle model: the position of its exhaust `[smoke]` emitter (M-0117).

use super::ModelConfig;
use bevy::prelude::*;

use crate::coords::omsi_to_bevy_pos;

/// The exhaust outlet in the vehicle root's local space (Bevy axes), read from the model's
/// `[smoke]` emitter. Only a vehicle whose model defines one has this component; the others use
/// the game's fallback outlet.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ExhaustOutlet(pub Vec3);

/// The exhaust outlet of `model`: the position of its first exhaust emitter, or `None` when the
/// model defines no such emitter.
#[must_use]
pub fn exhaust_outlet(model: &ModelConfig) -> Option<Vec3> {
    model
        .smokes
        .iter()
        .find(|s| {
            [&s.variable, &s.freq_variable]
                .into_iter()
                .flatten()
                .any(|name| is_exhaust_name(name))
        })
        .map(|s| omsi_to_bevy_pos(s.pos))
}

/// An emitter is the exhaust when its trigger or frequency variable names exhaust (`exhaust`
/// in any letter case). The variable names are read from the docs, not yet checked on a model.
fn is_exhaust_name(name: &str) -> bool {
    // OMSI names some exhaust emitters with `auspuff` (German for exhaust pipe).
    let lower = name.to_ascii_lowercase();
    lower.contains("exhaust") || lower.contains("auspuff")
}
