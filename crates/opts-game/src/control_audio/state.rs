//! `audio.state`: the engine's listener, the voice budget, the clips, the loudest loops and
//! the top emitters, as JSON.

use bevy::prelude::*;
use opts_bevy::audio::{AudioSettings, AudioStats};
use opts_bevy::control::state::{Outcome};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::audio_content::{ClipCache, ClipLoader};
use crate::player_sound::PlayerAudioStats;
use crate::scenery_sound::SceneryAudioStats;
use crate::vehicle_sound::AiAudioStats;

use super::bus_name;

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

pub(super) fn state(world: &mut World, _: &Value) -> Outcome {
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
