//! Stand-in gear model, AI variable publication, and 5 Hz voice streaming for AI traffic sounds.

use bevy::ecs::system::SystemParam;

use super::*;

use opts_vehicle_gears as gears;
mod variables;
pub use gears::{ai_engine_rpm, ai_gear};
pub(super) use variables::ai_loop;
pub use variables::{level_of, publish_sound_variables, speed_of};

/// The clip loader and the counters of the AI voices.
#[derive(SystemParam)]
pub struct AiVoiceState<'w> {
    loader: ResMut<'w, ClipLoader>,
    stats: ResMut<'w, AiAudioStats>,
}

/// Starts, updates and stops the emitters of AI voices five times a second.
pub fn stream_ai_voices(
    time: Res<Time>,
    listener: Query<&GlobalTransform, With<MainCamera>>,
    mut voices: Query<(
        Entity,
        &AiVoice,
        &GlobalTransform,
        &ChildOf,
        Option<&mut SoundEmitter>,
    )>,
    vehicles: Query<&AiVariables>,
    state: AiVoiceState,
    mut since: Local<f32>,
    mut commands: Commands,
) {
    let AiVoiceState {
        mut loader,
        mut stats,
    } = state;
    *since += time.delta_secs();
    if *since < 0.2 {
        return;
    }
    *since = 0.0;
    let Some(listener) = listener.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    let (mut total, mut playing) = (0, 0);
    for (entity, voice, tf, parent, emitter) in &mut voices {
        total += 1;
        let Ok(vars) = vehicles.get(voice.owner.unwrap_or_else(|| parent.parent())) else {
            continue;
        };
        let curve = SceneryVoice {
            def: voice.def.clone(),
            file: None,
            playback: crate::sound_logic::Playback::Loop,
            was_active: false,
        }
        .distance();
        let d = listener.distance(tf.translation());
        match emitter {
            Some(mut e) => {
                if d > curve.max * 1.25 {
                    commands.entity(entity).remove::<SoundEmitter>();
                } else {
                    playing += 1;
                    let (gain, speed) = ai_loop(&voice.def, vars);
                    if (e.gain - gain).abs() > 1e-3 {
                        e.gain = gain;
                    }
                    if (e.speed - speed).abs() > 1e-3 {
                        e.speed = speed;
                    }
                }
            }
            None if d <= curve.max => {
                let (gain, speed) = ai_loop(&voice.def, vars);
                if gain > 0.0 {
                    if let ClipState::Ready(clip) = loader.state(&voice.file) {
                        commands.entity(entity).insert(
                            SoundEmitter::new(clip, SoundBus::Vehicles)
                                .looping()
                                .spatial(curve)
                                .with_speed(speed)
                                .with_gain(gain),
                        );
                    }
                }
            }
            None => {}
        }
    }
    stats.voices = total;
    stats.playing = playing;
}

/// Registers the AI sound systems.
#[derive(Default)]
pub struct AiAudioPlugin;

impl Plugin for AiAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClipLoader>()
            .init_resource::<AiAudioStats>()
            .add_systems(
                Update,
                (stream_ai_voices, fire_ai_events).after(crate::audio_content::poll_clip_loader),
            );
    }
}
