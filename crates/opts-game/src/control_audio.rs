//! Control protocol of the sound engine (M-0027): `audio.state`, `audio.tone`, `audio.play` and
//! `audio.stop`. They make the engine testable on a machine where nobody listens: the state shows
//! the listener, the voice budget and what the loudest emitters compute, the tone and a played
//! file prove the output path.

mod play;
mod state;

use play::{play, tone};
use state::state;

use bevy::prelude::*;
use opts_bevy::audio::{SoundBus};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use serde_json::{json, Value};


/// Marks the emitters `audio.tone` and `audio.play` create, so `audio.stop` removes only them.
#[derive(Component)]
pub struct ProtocolSound;

/// Registers the methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("audio.state", state);
    registry.register("audio.tone", tone);
    registry.register("audio.play", play);
    registry.register("audio.stop", stop);
}

pub(super) fn bus_name(b: SoundBus) -> &'static str {
    match b {
        SoundBus::Effects => "effects",
        SoundBus::Ambient => "ambient",
        SoundBus::Vehicles => "vehicles",
        SoundBus::Ui => "ui",
    }
}

fn stop(world: &mut World, _: &Value) -> Outcome {
    let ids: Vec<Entity> = world
        .query_filtered::<Entity, With<ProtocolSound>>()
        .iter(world)
        .collect();
    let n = ids.len();
    for e in ids {
        world.despawn(e);
    }
    Outcome::ok(json!({ "stopped": n }))
}
