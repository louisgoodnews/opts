//! The streaming system of the player's voices: runs twenty times a second and updates loops and
//! edge-triggered voices.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_bevy::audio::SoundEmitter;
use opts_bevy::VehicleScripts;

use crate::audio_content::ClipLoader;
use crate::sound_logic::{conditions_hold, viewpoint_allows, Playback};

use super::listener::{listener_in_cabin, PlayerListener};
use super::steps::{edge_step, loop_step, variable};
use super::types::{PlayerAudioStats, PlayerVoice};

type VoiceQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut PlayerVoice,
        &'static GlobalTransform,
        &'static ChildOf,
        Option<&'static mut SoundEmitter>,
    ),
>;

/// The clip loader and the counters of the player's voices.
#[derive(SystemParam)]
pub struct PlayerVoiceState<'w> {
    pub(super) loader: ResMut<'w, ClipLoader>,
    pub(super) stats: ResMut<'w, PlayerAudioStats>,
}

/// Updates the loops and the `[noloop]` edges twenty times a second.
pub fn stream_player_voices(
    time: Res<Time>,
    ears: PlayerListener,
    mut voices: VoiceQuery,
    vehicles: Query<(&VehicleScripts, &GlobalTransform)>,
    state: PlayerVoiceState,
    mut since: Local<f32>,
    mut commands: Commands,
) {
    let PlayerListener { camera, listener } = ears;
    let PlayerVoiceState {
        mut loader,
        mut stats,
    } = state;
    *since += time.delta_secs();
    if *since < 0.05 {
        return;
    }
    let dt = std::mem::take(&mut *since);
    let Some(listener) = listener.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    let (mut total, mut loops, mut inside_any) = (0, 0, false);
    for (entity, mut voice, tf, parent, emitter) in &mut voices {
        total += 1;
        let Ok((scripts, vehicle_tf)) =
            vehicles.get(voice.owner.unwrap_or_else(|| parent.parent()))
        else {
            continue;
        };
        let inside = listener_in_cabin(camera.as_ref().map(|c| c.mode), vehicle_tf, listener);
        inside_any |= inside;
        let Some(active) = conditions_hold(&voice.def.conditions, |n: &str| variable(scripts, n))
        else {
            continue;
        };
        let audible_here = viewpoint_allows(voice.def.viewpoint, inside);
        // How long it has been sounding: the volume curves of the starter and retarder loops fade in
        // over it (`[volcurve] -1`).
        if active && audible_here {
            voice.active_s += dt;
        } else {
            voice.active_s = 0.0;
        }
        let age = voice.active_s;
        let read = |n: &str| {
            if n == crate::sound_logic::AGE_VARIABLE {
                age
            } else {
                variable(scripts, n)
            }
        };
        match voice.playback {
            Playback::Loop => {
                let d = listener.distance(tf.translation());
                loops += usize::from(loop_step(
                    &mut commands,
                    &mut loader,
                    (entity, &mut voice, emitter.as_deref()),
                    (d, active && audible_here),
                    read,
                ));
            }
            Playback::OnEdge => edge_step(
                &mut commands,
                (&mut loader, &mut stats),
                (entity, &mut voice),
                (active, audible_here),
                read,
            ),
            Playback::OnTrigger | Playback::Unsupported => {}
        }
    }
    stats.voices = total;
    stats.loops = loops;
    stats.inside = inside_any;
}
