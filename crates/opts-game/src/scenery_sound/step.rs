//! Per-frame steps of the scenery voices: looping and edge-triggered.


use super::*;


pub(super) type VoiceQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut SceneryVoice,
        &'static GlobalTransform,
        Option<&'static mut SoundEmitter>,
    ),
>;

/// A looping voice: stops its emitter when the listener is far away, follows the conditions with
/// the gain, and starts the emitter when it comes near and its conditions hold. Returns 1 when
/// the voice has an emitter that keeps playing.
pub(super) fn loop_step(
    commands: &mut Commands,
    loader: &mut ClipLoader,
    (entity, voice, emitter): (Entity, &SceneryVoice, Option<Mut<SoundEmitter>>),
    (path, d, active): (&PathBuf, f32, bool),
    read: impl Fn(&str) -> f32 + Copy,
) -> usize {
    let curve = voice.distance();
    let mut playing = 0;
    match emitter {
        Some(mut e) => {
            if d > curve.max * 1.25 {
                commands.entity(entity).remove::<SoundEmitter>();
            } else {
                playing += 1;
                let gain = if active { level(&voice.def, read) } else { 0.0 };
                if (e.gain - gain).abs() > 1e-3 {
                    e.gain = gain;
                }
            }
        }
        None if d <= curve.max && active => {
            let (gain, spd) = crate::sound_logic::loop_voice(&voice.def, read);
            if gain > 0.0 {
                if let ClipState::Ready(clip) = loader.state(path) {
                    commands.entity(entity).insert(
                        SoundEmitter::new(clip, SoundBus::Ambient)
                            .looping()
                            .spatial(curve)
                            .with_speed(spd)
                            .with_gain(gain),
                    );
                }
            }
        }
        None => {}
    };
    playing
}

/// An edge-triggered voice: plays a one-shot when its conditions just turned true and the
/// listener is in range; counts a cold miss while the clip still loads.
pub(super) fn edge_step(
    commands: &mut Commands,
    (loader, stats): (&mut ClipLoader, &mut SceneryAudioStats),
    (entity, voice, path): (Entity, &mut SceneryVoice, &PathBuf),
    (d, active): (f32, bool),
    read: impl Fn(&str) -> f32 + Copy,
) {
    let curve = voice.distance();
    let edge = active && !voice.was_active;
    if voice.was_active != active {
        voice.was_active = active;
    }
    if edge && d <= curve.max {
        match loader.state(path) {
            ClipState::Ready(clip) => {
                stats.one_shots += 1;
                commands.entity(entity).with_children(|p| {
                    p.spawn((
                        SoundEmitter::new(clip, SoundBus::Ambient)
                            .spatial(curve)
                            .with_speed(voice.def.pitch.clamp(0.3, 3.5))
                            .with_gain(level(&voice.def, read)),
                        Transform::IDENTITY,
                        Name::new("scenery one-shot"),
                    ));
                });
            }
            ClipState::Loading => stats.cold_misses += 1,
            ClipState::Failed => {}
        }
    }
}
