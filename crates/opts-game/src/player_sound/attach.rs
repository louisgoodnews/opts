//! Building the voices of a vehicle from its `[sound]` configuration, and the script-driven ones.

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::omsi_to_bevy_pos;
use opts_core::formats::sound::SoundDef;
use opts_core::formats::vehicles::BusManifest;

use crate::in_game::omsi_world::WorldLoadJob;
use crate::sound_logic::{playback_of, Playback};

use super::types::{PlayerSoundInfo, PlayerVoice};

/// Adds the voices of the vehicle's `[sound]` configuration under `root`.
pub fn attach(
    job: &WorldLoadJob,
    commands: &mut Commands,
    root: Entity,
    manifest: &BusManifest,
    bus_dir: &std::path::Path,
) {
    attach_unit(job, commands, root, None, manifest, bus_dir);
}

/// Adds the voices of the `[sound]` configuration of one unit of a vehicle under `unit`. A trailer
/// (`owner` is the head) has its own configuration at its own places but the head's variables and
/// events: the engine of an articulated bus can sit in its second unit (the Bremen MAN NG313's
/// engine loops are in `sound_NG313_T.cfg`).
pub fn attach_unit(
    job: &WorldLoadJob,
    commands: &mut Commands,
    unit: Entity,
    owner: Option<Entity>,
    manifest: &BusManifest,
    bus_dir: &std::path::Path,
) {
    let root = unit;
    let Some(rel) = manifest.sound_cfg.as_deref() else {
        return;
    };
    let Some((cfg, cfg_dir)) = job.read_sound_config(bus_dir, rel) else {
        tracing::warn!(cfg = rel, "the vehicle's sound configuration was not found");
        return;
    };
    let mut info = PlayerSoundInfo {
        total: cfg.sounds.len(),
        ..Default::default()
    };
    let mut voices = Vec::new();
    for def in &cfg.sounds {
        let playback = playback_of(def);
        if playback == Playback::Unsupported {
            info.unsupported += 1;
            continue;
        }
        let Some(file) = job.find_sound_file(&cfg_dir, &def.file) else {
            info.unresolved += 1;
            continue;
        };
        let at = def.position.unwrap_or([0.0; 3]);
        voices.push((
            PlayerVoice {
                def: Arc::new(def.clone()),
                file,
                playback,
                was_active: false,
                owner,
                goal: None,
                active_s: 0.0,
            },
            omsi_to_bevy_pos(Vec3::new(at[0], at[1], at[2])),
        ));
    }
    tracing::info!(
        total = info.total,
        voices = voices.len(),
        unsupported = info.unsupported,
        unresolved = info.unresolved,
        "player vehicle sounds"
    );
    commands.entity(root).insert(info).with_children(|p| {
        for (voice, pos) in voices {
            p.spawn((
                voice,
                Transform::from_translation(pos),
                Name::new("PlayerVoice"),
            ));
        }
    });
}

/// Adds script-driven voices (definitions with the vehicle's own variables and events) under
/// `unit`: for a scheduled AI bus, whose door scripts raise `ev_DoorTrigger…` and set `door_N`
/// (the AI configuration's door sounds). The unit must carry the scripts.
pub fn attach_defs(
    commands: &mut Commands,
    unit: Entity,
    owner: Option<Entity>,
    defs: &[(Arc<SoundDef>, PathBuf)],
) -> usize {
    let mut voices = Vec::new();
    for (def, file) in defs {
        let playback = playback_of(def);
        if playback == Playback::Unsupported {
            continue;
        }
        let at = def.position.unwrap_or([0.0; 3]);
        voices.push((
            PlayerVoice {
                def: def.clone(),
                file: file.clone(),
                playback,
                was_active: false,
                owner,
                goal: None,
                active_s: 0.0,
            },
            omsi_to_bevy_pos(Vec3::new(at[0], at[1], at[2])),
        ));
    }
    let n = voices.len();
    commands.entity(unit).with_children(|p| {
        for (voice, pos) in voices {
            p.spawn((
                voice,
                Transform::from_translation(pos),
                Name::new("AiScriptVoice"),
            ));
        }
    });
    n
}
