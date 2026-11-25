//! Voice selection: the budget rule that gives the loudest emitters a voice.

use super::*;

/// [`AUDIBLE_MIN`]. Pure; the engine's budget rule.
#[must_use]
pub fn choose_voices(levels: &[(usize, f32)], budget: usize) -> Vec<usize> {
    let mut audible: Vec<(usize, f32)> = levels
        .iter()
        .copied()
        .filter(|(_, g)| *g >= AUDIBLE_MIN)
        .collect();
    audible.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    audible.truncate(budget);
    audible.into_iter().map(|(i, _)| i).collect()
}

/// Decides every frame which emitters are heard: computes their level, gives the loudest ones
/// a voice (an [`AudioPlayer`]) and takes it away from the others, and despawns one-shots that
/// finished or could not be heard when they were triggered.
pub fn select_voices(
    settings: Res<AudioSettings>,
    listener: Query<&GlobalTransform, With<MainCamera>>,
    mut emitters: EmitterQuery,
    mut stats: ResMut<AudioStats>,
    mut commands: Commands,
) {
    let listener_pos = listener.iter().next().map(GlobalTransform::translation);
    stats.listener = listener_pos;
    let all = emitter_levels(&settings, listener_pos, &emitters);
    let levels: Vec<(usize, f32)> = all
        .iter()
        .enumerate()
        .map(|(i, (_, g, _, _))| (i, *g))
        .collect();
    let chosen: std::collections::HashSet<usize> = choose_voices(&levels, settings.max_voices)
        .into_iter()
        .collect();

    let (mut audible, mut voices) = (0, 0);
    let mut report: Vec<EmitterReport> = Vec::new();
    for (i, (entity, gain, distance, position)) in all.iter().enumerate() {
        let Ok((_, e, _, voice, has_player)) = emitters.get_mut(*entity) else {
            continue;
        };
        if *gain >= AUDIBLE_MIN {
            audible += 1;
        }
        let want = chosen.contains(&i);
        let has_voice = voice.is_some();
        match (want, has_voice, has_player) {
            // A new voice.
            (true, false, _) => {
                voices += 1;
                start_voice(&mut commands, *entity, e, *gain);
            }
            // A running voice: keep it, remember its level.
            (true, true, true) => {
                voices += 1;
                if let Some(mut v) = voice {
                    v.gain = *gain;
                }
            }
            // A one-shot whose player finished and was removed by Bevy.
            (_, true, false) => {
                commands.entity(*entity).remove::<Voice>();
                if e.despawn_when_done && !e.looping {
                    commands.entity(*entity).despawn();
                }
            }
            // Lost the slot or fell silent: stop the voice.
            (false, true, true) => stop_voice(&mut commands, *entity, e),
            // Never had a voice and has none: a one-shot nobody can hear is dropped at once.
            (false, false, _) => {
                if e.despawn_when_done && !e.looping {
                    commands.entity(*entity).despawn();
                }
            }
        }
        report.push(EmitterReport {
            entity: *entity,
            gain: *gain,
            distance: *distance,
            voice: want,
            bus: e.bus,
            position: *position,
        });
    }
    report.sort_by(|a, b| b.gain.total_cmp(&a.gain));
    report.truncate(16);
    stats.emitters = all.len();
    stats.audible = audible;
    stats.voices = voices;
    stats.virtualized = audible.saturating_sub(voices);
    stats.top = report;
}
