//! `audio.tone` and `audio.play`: the protocol sounds (a sine tone, or an OMSI sound file) and
//! the emitters they spawn.

use bevy::prelude::*;
use opts_bevy::audio::{Distance, SoundBus, SoundEmitter};
use opts_bevy::control::params::{opt_bool, opt_f64, opt_str, str_param};
use opts_bevy::control::state::Outcome;
use opts_bevy::control::PathPolicy;
use opts_bevy::PcmClip;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::audio_content::ClipCache;

use super::{bus_name, ProtocolSound};

fn bus_of(name: Option<&str>) -> Result<SoundBus, RpcError> {
    Ok(match name {
        None | Some("effects") => SoundBus::Effects,
        Some("ambient") => SoundBus::Ambient,
        Some("vehicles") => SoundBus::Vehicles,
        Some("ui") => SoundBus::Ui,
        Some(_) => {
            return Err(RpcError::invalid_params(
                "`bus` is effects, ambient, vehicles or ui",
            ))
        }
    })
}

/// A position given as `at: [x, y, z]` in Bevy world coordinates (metres), with the distance
/// curve from `reference` and `max` (defaults 5 and 120 m).
fn spatial_of(params: &Value) -> Result<Option<(Vec3, Distance)>, RpcError> {
    let Some(at) = params.get("at") else {
        return Ok(None);
    };
    let a = at
        .as_array()
        .filter(|a| a.len() == 3)
        .and_then(|a| {
            Some(Vec3::new(
                a[0].as_f64()? as f32,
                a[1].as_f64()? as f32,
                a[2].as_f64()? as f32,
            ))
        })
        .ok_or_else(|| RpcError::invalid_params("`at` is [x, y, z] in metres"))?;
    let reference = opt_f64(params, "reference")?.unwrap_or(5.0) as f32;
    let max = opt_f64(params, "max")?.unwrap_or(120.0) as f32;
    Ok(Some((a, Distance::new(reference, max))))
}

fn spawn_sound(
    world: &mut World,
    clip: Handle<PcmClip>,
    params: &Value,
) -> Result<Value, RpcError> {
    let bus = bus_of(opt_str(params, "bus")?)?;
    let looping = opt_bool(params, "loop")?.unwrap_or(false);
    let gain = opt_f64(params, "gain")?.unwrap_or(1.0) as f32;
    let speed = opt_f64(params, "speed")?.unwrap_or(1.0) as f32;
    let mut emitter = SoundEmitter::new(clip, bus)
        .with_gain(gain)
        .with_speed(speed);
    if looping {
        emitter = emitter.looping();
    }
    let at = spatial_of(params)?;
    let mut e = world.spawn((ProtocolSound, Name::new("protocol sound")));
    if let Some((pos, curve)) = at {
        emitter = emitter.spatial(curve);
        e.insert(Transform::from_translation(pos));
    }
    // A protocol sound that is not heard (volume 0, too far) is kept for `audio.state` to show.
    emitter.despawn_when_done = false;
    e.insert(emitter);
    Ok(json!({ "entity": e.id().to_bits(), "bus": bus_name(bus), "loop": looping }))
}

/// `audio.tone {frequency?, seconds?, gain?, bus?, loop?, at?}`: a sine tone.
pub(super) fn tone(world: &mut World, params: &Value) -> Outcome {
    let run = |world: &mut World| -> Result<Value, RpcError> {
        let frequency = opt_f64(params, "frequency")?.unwrap_or(440.0) as f32;
        let seconds = opt_f64(params, "seconds")?.unwrap_or(1.0) as f32;
        if !(20.0..=20_000.0).contains(&frequency) || !(0.05..=30.0).contains(&seconds) {
            return Err(RpcError::invalid_params(
                "`frequency` is 20 to 20000 Hz and `seconds` 0.05 to 30",
            ));
        }
        let clip = PcmClip::tone(frequency, seconds, 44_100);
        let handle = world
            .get_resource_mut::<Assets<PcmClip>>()
            .ok_or_else(|| RpcError::new(code::NOT_READY, "the sound engine is not running"))?
            .add(clip);
        spawn_sound(world, handle, params)
    };
    run(world).into()
}

/// `audio.play {file, gain?, speed?, bus?, loop?, at?}`: an OMSI sound file.
pub(super) fn play(world: &mut World, params: &Value) -> Outcome {
    let run = |world: &mut World| -> Result<Value, RpcError> {
        let file = str_param(params, "file")?;
        let path = world
            .get_resource::<PathPolicy>()
            .ok_or_else(|| RpcError::new(code::NOT_READY, "no file policy"))?
            .check_read(std::path::Path::new(file))?;
        let handle = world.resource_scope(|world, mut cache: Mut<ClipCache>| {
            let mut assets = world.resource_mut::<Assets<PcmClip>>();
            cache.clip(&path, &mut assets)
        });
        let handle = handle.map_err(|e| RpcError::new(code::FAILED, e))?;
        spawn_sound(world, handle, params)
    };
    run(world).into()
}
