//! One-shot triggers, clip preloading, and gain/speed smoothing for player vehicle sounds.

use super::*;

/// Starts decoding the files of the one-shots (doors, switches, brakes, the horn's start and end)
/// as soon as the voices exist: the first ask of a clip that is still undecoded is silent
/// (`ClipLoader`), and a door that opens for the first time must be heard.
pub fn preload_one_shots(
    voices: Query<&PlayerVoice, Added<PlayerVoice>>,
    mut loader: ResMut<ClipLoader>,
) {
    for voice in &voices {
        if matches!(voice.playback, Playback::OnTrigger | Playback::OnEdge) {
            let _ = loader.state(&voice.file);
        }
    }
}

/// How fast a loop's gain and speed follow their goals (seconds to cover about two thirds of the
/// way): short enough to follow the engine, long enough to hide the 20 Hz updates.
const GLIDE_S: f32 = 0.08;

/// Moves the gain and speed of the voices' emitters towards their goals every frame.
pub fn smooth_player_voices(
    time: Res<Time>,
    mut voices: Query<(&mut PlayerVoice, &mut SoundEmitter)>,
) {
    let k = 1.0 - (-time.delta_secs() / GLIDE_S).exp();
    for (voice, mut emitter) in &mut voices {
        let Some((gain, speed)) = voice.goal else {
            continue;
        };
        let step = |now: f32, goal: f32| {
            if (goal - now).abs() < 1e-3 {
                goal
            } else {
                now + (goal - now) * k
            }
        };
        let (g, s) = (step(emitter.gain, gain), step(emitter.speed, speed));
        if (emitter.gain - g).abs() > f32::EPSILON {
            emitter.gain = g;
        }
        if (emitter.speed - s).abs() > f32::EPSILON {
            emitter.speed = s;
        }
    }
}

/// Starts a one-shot of `voice`: a child emitter at its place, gone when it has played.
pub(super) fn fire(
    commands: &mut Commands,
    loader: &mut ClipLoader,
    stats: &mut PlayerAudioStats,
    entity: Entity,
    voice: &PlayerVoice,
    level: f32,
) {
    match loader.state(&voice.file) {
        ClipState::Ready(clip) => {
            stats.one_shots += 1;
            let emitter = SoundEmitter::new(clip, SoundBus::Effects)
                .spatial(voice.distance())
                .with_speed(voice.def.pitch.clamp(0.3, 3.5))
                .with_gain(level);
            commands.entity(entity).with_children(|p| {
                p.spawn((emitter, Transform::IDENTITY, Name::new("player one-shot")));
            });
        }
        // The first ask only starts the decode; the next one is heard.
        ClipState::Loading => stats.cold_misses += 1,
        ClipState::Failed => {}
    }
}

/// Plays the one-shots whose `[trigger]` the vehicle's scripts raised this frame.
pub fn fire_player_triggers(
    camera: Option<Res<opts_bevy::CameraState>>,
    listener: Query<&GlobalTransform, With<MainCamera>>,
    mut vehicles: Query<(Entity, &mut VehicleScripts, &GlobalTransform)>,
    voices: Query<(Entity, &PlayerVoice, &ChildOf)>,
    mut loader: ResMut<ClipLoader>,
    mut stats: ResMut<PlayerAudioStats>,
    mut commands: Commands,
) {
    let listener = listener.iter().next().map(GlobalTransform::translation);
    for (vehicle, mut scripts, vehicle_tf) in &mut vehicles {
        let raised = {
            let s = scripts.bypass_change_detection();
            if s.raised.is_empty() {
                continue;
            }
            s.take_raised()
        };
        let inside = listener
            .is_some_and(|l| listener_in_cabin(camera.as_ref().map(|c| c.mode), vehicle_tf, l));
        for name in raised {
            let mut started = 0;
            // The voices of the vehicle: on its root, and on the units behind it that share its
            // scripts.
            for (child, voice, parent) in &voices {
                if voice.owner.unwrap_or_else(|| parent.parent()) != vehicle {
                    continue;
                }
                if voice.playback != Playback::OnTrigger
                    || !voice
                        .def
                        .triggers
                        .iter()
                        .any(|t| t.trim().eq_ignore_ascii_case(&name))
                    || !viewpoint_allows(voice.def.viewpoint, inside)
                {
                    continue;
                }
                let read = |n: &str| variable(&scripts, n);
                if conditions_hold(&voice.def.conditions, read) != Some(true) {
                    continue;
                }
                let lvl = level(&voice.def, read);
                fire(&mut commands, &mut loader, &mut stats, child, voice, lvl);
                started += 1;
            }
            if stats.events.len() >= 64 {
                stats.events.pop_front();
            }
            stats.events.push_back((name, started));
        }
    }
}

/// Registers the player vehicle sound systems.
#[derive(Default)]
pub struct PlayerAudioPlugin;

impl Plugin for PlayerAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClipLoader>()
            .init_resource::<PlayerAudioStats>()
            .add_systems(
                Update,
                (
                    stream_player_voices,
                    fire_player_triggers,
                    smooth_player_voices.after(stream_player_voices),
                    preload_one_shots,
                )
                    .after(crate::audio_content::poll_clip_loader),
            );
    }
}
