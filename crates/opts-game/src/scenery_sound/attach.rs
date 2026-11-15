//! Attaches the sound voices of a placed scenery object: the `[sound]` configuration of its `.sco`
//! becomes [`SceneryVoice`] children.

use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::omsi_to_bevy_pos;
use opts_core::formats::scenery::SceneryObject;

use crate::in_game::omsi_world::WorldLoadJob;
use crate::sound_logic::playback_of;

use super::SceneryVoice;

/// Adds the voices of `sco`'s sound configuration under `root`.
pub fn attach(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    root: Entity,
    sco: &SceneryObject,
    sco_dir: &std::path::Path,
) {
    let Some(rel) = sco.sound.as_deref() else {
        return;
    };
    let Some((cfg, cfg_dir)) = job.sound_config(sco_dir, rel) else {
        tracing::debug!(sco_dir = %sco_dir.display(), cfg = rel, "sound configuration not found");
        return;
    };
    let voices: Vec<(SceneryVoice, Vec3)> = cfg
        .sounds
        .iter()
        .map(|def| {
            let p = def.position.unwrap_or([0.0; 3]);
            (
                SceneryVoice {
                    file: job.find_sound_file(&cfg_dir, &def.file),
                    playback: playback_of(def),
                    was_active: false,
                    def: Arc::new(def.clone()),
                },
                omsi_to_bevy_pos(Vec3::new(p[0], p[1], p[2])),
            )
        })
        .collect();
    commands.entity(root).with_children(|parent| {
        for (voice, pos) in voices {
            parent.spawn((
                voice,
                Transform::from_translation(pos),
                Name::new("SceneryVoice"),
            ));
        }
    });
}
