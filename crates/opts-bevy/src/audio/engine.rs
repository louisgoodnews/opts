//! The sound engine's systems: listener, voice selection, level sync and the statistics the
//! control protocol reports (`audio.state`).

use bevy::audio::{
    AudioPlayer, AudioSink, AudioSinkPlayback, PlaybackMode, PlaybackSettings, SpatialAudioSink,
    SpatialListener, SpatialScale, Volume,
};
use bevy::prelude::*;

use super::clip::PcmClip;
use super::curves::percent_to_gain;
use super::emitter::{SoundBus, SoundEmitter, Voice};
use crate::MainCamera;

/// The volume settings (percent) and limits; the game keeps it in step with its settings.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct AudioSettings {
    /// Sound on or off altogether.
    pub enabled: bool,
    /// Master volume, 0 to 100.
    pub master: f32,
    /// Effects bus, 0 to 100.
    pub effects: f32,
    /// Ambient bus, 0 to 100.
    pub ambient: f32,
    /// Vehicles bus, 0 to 100.
    pub vehicles: f32,
    /// Interface bus, 0 to 100.
    pub ui: f32,
    /// Most voices playing at once (the loudest win).
    pub max_voices: usize,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            master: 80.0,
            effects: 80.0,
            ambient: 60.0,
            vehicles: 80.0,
            ui: 60.0,
            max_voices: 32,
        }
    }
}

impl AudioSettings {
    /// The linear gain of a bus including the master volume.
    #[must_use]
    pub fn bus_gain(&self, bus: SoundBus) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        let percent = match bus {
            SoundBus::Effects => self.effects,
            SoundBus::Ambient => self.ambient,
            SoundBus::Vehicles => self.vehicles,
            SoundBus::Ui => self.ui,
        };
        percent_to_gain(self.master) * percent_to_gain(percent)
    }
}

/// Levels below this are not worth a voice.
pub const AUDIBLE_MIN: f32 = 0.002;

/// One emitter as the statistics list it.
#[derive(Debug, Clone, PartialEq)]
pub struct EmitterReport {
    /// The entity.
    pub entity: Entity,
    /// Computed level (source × distance × bus × master).
    pub gain: f32,
    /// Metres from the listener (`None` for a source that is not spatial).
    pub distance: Option<f32>,
    /// Whether it has a playing voice.
    pub voice: bool,
    /// The bus.
    pub bus: SoundBus,
}

/// What the engine did last frame.
#[derive(Resource, Debug, Clone, Default)]
pub struct AudioStats {
    /// All emitters.
    pub emitters: usize,
    /// Emitters loud enough to be heard.
    pub audible: usize,
    /// Emitters with a voice.
    pub voices: usize,
    /// Audible emitters left without a voice by the budget.
    pub virtualized: usize,
    /// Voices whose sink exists (the sound card accepted them).
    pub playing: usize,
    /// The listener's position, if there is one.
    pub listener: Option<Vec3>,
    /// The loudest emitters (at most 16).
    pub top: Vec<EmitterReport>,
}

/// Puts a [`SpatialListener`] on the main camera (a head about 0.2 m wide).
pub fn ensure_listener(
    cameras: Query<Entity, (With<MainCamera>, Without<SpatialListener>)>,
    mut commands: Commands,
) {
    for e in &cameras {
        commands.entity(e).insert(SpatialListener::new(0.2));
    }
}

/// Which of `levels` (index, gain) get a voice: the loudest `budget`, none below
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

type EmitterQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static SoundEmitter,
        Option<&'static GlobalTransform>,
        Option<&'static mut Voice>,
        Has<AudioPlayer<PcmClip>>,
    ),
>;

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
    let all: Vec<(Entity, f32, Option<f32>)> = emitters
        .iter()
        .map(|(entity, e, tf, _, _)| {
            let (distance, spatial_gain) = match (&e.spatial, listener_pos, tf) {
                (Some(curve), Some(l), Some(t)) => {
                    let d = l.distance(t.translation());
                    (Some(d), curve.gain(d))
                }
                // A spatial source with no listener or no position cannot be heard.
                (Some(_), _, _) => (None, 0.0),
                (None, _, _) => (None, 1.0),
            };
            let gain = if e.muted {
                0.0
            } else {
                e.gain * spatial_gain * settings.bus_gain(e.bus)
            };
            (entity, gain, distance)
        })
        .collect();
    let levels: Vec<(usize, f32)> = all
        .iter()
        .enumerate()
        .map(|(i, (_, g, _))| (i, *g))
        .collect();
    let chosen: std::collections::HashSet<usize> = choose_voices(&levels, settings.max_voices)
        .into_iter()
        .collect();

    let (mut audible, mut voices) = (0, 0);
    let mut report: Vec<EmitterReport> = Vec::new();
    for (i, (entity, gain, distance)) in all.iter().enumerate() {
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
                commands.entity(*entity).insert((
                    AudioPlayer::<PcmClip>(e.clip.clone()),
                    PlaybackSettings {
                        mode: if e.looping {
                            PlaybackMode::Loop
                        } else {
                            PlaybackMode::Remove
                        },
                        volume: Volume::Linear(*gain),
                        speed: e.speed,
                        spatial: e.spatial.is_some(),
                        // Bevy's own distance fall-off (1/d²) is kept out of the way: positions
                        // are scaled into the unit range, our curve does the fading.
                        spatial_scale: e.spatial.map(|d| SpatialScale::new(1.0 / d.max.max(1.0))),
                        ..PlaybackSettings::ONCE
                    },
                    Voice { gain: *gain },
                ));
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
            (false, true, true) => {
                commands.entity(*entity).remove::<(
                    AudioPlayer<PcmClip>,
                    PlaybackSettings,
                    AudioSink,
                    SpatialAudioSink,
                    Voice,
                )>();
                if e.despawn_when_done && !e.looping {
                    commands.entity(*entity).despawn();
                }
            }
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

/// Applies the computed level and the emitter's speed to the sinks of the running voices.
pub fn sync_voice_levels(
    mut voices: Query<(
        &Voice,
        &SoundEmitter,
        Option<&mut AudioSink>,
        Option<&mut SpatialAudioSink>,
    )>,
    mut stats: ResMut<AudioStats>,
) {
    let mut playing = 0;
    for (voice, emitter, plain, spatial) in &mut voices {
        let volume = Volume::Linear(voice.gain);
        if let Some(mut s) = plain {
            s.set_volume(volume);
            s.set_speed(emitter.speed);
            playing += 1;
        }
        if let Some(mut s) = spatial {
            s.set_volume(volume);
            s.set_speed(emitter.speed);
            playing += 1;
        }
    }
    stats.playing = playing;
}
