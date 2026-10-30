//! Control protocol of the sound engine (M-0027): `audio.state`, `audio.tone`, `audio.play` and
//! `audio.stop`. They make the engine testable on a machine where nobody listens: the state shows
//! the listener, the voice budget and what the loudest emitters compute, the tone and a played
//! file prove the output path.

use bevy::prelude::*;
use opts_bevy::audio::{AudioSettings, AudioStats, Distance, SoundBus, SoundEmitter};
use opts_bevy::control::params::{opt_bool, opt_f64, opt_str, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::control::PathPolicy;
use opts_bevy::PcmClip;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::audio_content::{ClipCache, ClipLoader};
use crate::player_sound::PlayerAudioStats;
use crate::scenery_sound::SceneryAudioStats;
use crate::vehicle_sound::AiAudioStats;

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

fn bus_name(b: SoundBus) -> &'static str {
    match b {
        SoundBus::Effects => "effects",
        SoundBus::Ambient => "ambient",
        SoundBus::Vehicles => "vehicles",
        SoundBus::Ui => "ui",
    }
}

/// What the player's vehicle is sounding: the loudest loops with the speed they play at, so the
/// engine layers (idle, throttle, load, interior and exterior) can be told apart.
fn loudest_loops(world: &mut World) -> Vec<Value> {
    let mut loud: Vec<(f32, f32, String)> = world
        .query::<(
            &crate::player_sound::PlayerVoice,
            &opts_bevy::audio::SoundEmitter,
        )>()
        .iter(world)
        .filter(|(_, e)| e.gain > 0.001 && !e.muted)
        .map(|(v, e)| {
            let file = v
                .file
                .file_name()
                .map_or_else(String::new, |f| f.to_string_lossy().into_owned());
            (e.gain, e.speed, file)
        })
        .collect();
    loud.sort_by(|a, b| b.0.total_cmp(&a.0));
    loud.iter()
        .take(10)
        .map(|(gain, speed, file)| json!({ "file": file, "gain": gain, "speed": speed }))
        .collect()
}

/// The loudest emitters the engine keeps statistics of.
fn top_voices(stats: &AudioStats) -> Vec<Value> {
    stats
        .top
        .iter()
        .map(|e| {
            json!({
                "entity": e.entity.to_bits(),
                "bus": bus_name(e.bus),
                "gain": e.gain,
                "distance": e.distance,
                "voice": e.voice,
                "position": e.position.map(|p| json!([p.x, p.y, p.z])),
            })
        })
        .collect()
}

fn state(world: &mut World, _: &Value) -> Outcome {
    let settings = world.get_resource::<AudioSettings>().cloned();
    let Some(settings) = settings else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "the sound engine is not running",
        ));
    };
    let stats = world
        .get_resource::<AudioStats>()
        .cloned()
        .unwrap_or_default();
    let cached = world.get_resource::<ClipCache>().map_or(0, ClipCache::len);
    let scenery = world
        .get_resource::<SceneryAudioStats>()
        .cloned()
        .unwrap_or_default();
    let player = world
        .get_resource::<PlayerAudioStats>()
        .cloned()
        .unwrap_or_default();
    let traffic = world
        .get_resource::<AiAudioStats>()
        .cloned()
        .unwrap_or_default();
    let (ready, loading, failed) = world
        .get_resource::<ClipLoader>()
        .map_or((0, 0, 0), ClipLoader::counts);
    let loaded_bytes = world.get_resource::<ClipLoader>().map_or(0, |l| l.bytes);
    let loudest = loudest_loops(world);
    let top = top_voices(&stats);
    Outcome::ok(json!({
        "enabled": settings.enabled,
        "volumes": {
            "master": settings.master, "effects": settings.effects, "ambience": settings.ambient,
            "engine": settings.vehicles, "interface": settings.ui,
        },
        "voices": {
            "budget": settings.max_voices,
            "emitters": stats.emitters,
            "audible": stats.audible,
            "with_voice": stats.voices,
            "virtualized": stats.virtualized,
            "playing": stats.playing,
        },
        "listener": stats.listener.map(|l| json!([l.x, l.y, l.z])),
        "clips_cached": cached,
        "clips": { "ready": ready, "loading": loading, "failed": failed, "bytes": loaded_bytes },
        "player": {
            "voices": player.voices,
            "loops": player.loops,
            "one_shots": player.one_shots,
            "cold_misses": player.cold_misses,
            "unsupported": player.unsupported,
            "files_not_found": player.unresolved,
            "listener_inside": player.inside,
            "loudest": loudest,
            "events": player.events.iter().map(|(n, k)| json!({ "event": n, "voices": k })).collect::<Vec<_>>(),
        },
        "traffic": { "voices": traffic.voices, "playing": traffic.playing },
        "scenery": {
            "voices": scenery.voices,
            "playing": scenery.playing,
            "one_shots": scenery.one_shots,
            "cold_misses": scenery.cold_misses,
            "unsupported_conditions": scenery.unsupported,
            "files_not_found": scenery.unresolved,
        },
        "top": top,
    }))
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
fn tone(world: &mut World, params: &Value) -> Outcome {
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
fn play(world: &mut World, params: &Value) -> Outcome {
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
