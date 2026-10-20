//! Sounds of scenery objects (M-0139): the `[sound]` configuration of a `.sco` becomes
//! [`SceneryVoice`] children of the placed object, and three systems turn them into sound:
//!
//! * **continuous voices** (a `[sound]` without `[trigger]`, `[conditionSingle]` or `[noloop]`:
//!   hums, ambience, birdsong; ❓ inferred, see `docs/src/formats/text/vehicles/sound_cfg.md`) get
//!   a looping [`SoundEmitter`] while the listener is within their range, with a level from their
//!   `[volcurve]`s over the session clock (`Time`, `DayOfYear`, `SunAlt`);
//! * **triggered voices** (`[trigger] name`) play a one-shot when the object's script raises that
//!   event (`(T.L.name)`), at the voice's place;
//! * clips are decoded on worker threads ([`ClipLoader`]) the first time a voice comes near.
//!
//! Not yet: `[conditionSingle]` (counted in [`SceneryAudioStats::unsupported`]), `[viewpoint]`,
//! `[onlyone]`, `[checkloading]`, and the objects' own script variables in `[volcurve]`.

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use opts_bevy::audio::{Distance, SoundBus, SoundEmitter};
use opts_bevy::scenery_runtime::{HostValues, SceneryClock};
use opts_bevy::{omsi_to_bevy_pos, MainCamera, SceneryScripts};
use opts_core::formats::scenery::SceneryObject;
use opts_core::formats::sound::SoundDef;

use crate::audio_content::{poll_clip_loader, ClipLoader, ClipState};
use crate::in_game::omsi_world::WorldLoadJob;
use crate::sound_logic::{conditions_hold, level, playback_of, Playback};

/// One sound definition of a placed object, a child at its `[3d]` position.
#[derive(Component, Debug, Clone)]
pub struct SceneryVoice {
    /// The definition.
    pub def: Arc<SoundDef>,
    /// The sound file, when it was found.
    pub file: Option<PathBuf>,
    /// What it does when active ([`crate::sound_logic::playback_of`]).
    pub playback: Playback,
    /// The conditions held at the last look (edge detection of `[noloop]` voices).
    pub was_active: bool,
}

impl SceneryVoice {
    /// How the voice fades: full level inside the `[3d]` range, silent beyond six times that
    /// (at least 40 m more). OMSI's own maximum distance is not known (Q-AU01).
    #[must_use]
    pub fn distance(&self) -> Distance {
        let r = self.def.range.max(0.5);
        Distance::new(r, (r * 6.0).max(r + 40.0))
    }
}

/// What the scenery sounds did, for `audio.state`.
#[derive(Resource, Debug, Clone, Default)]
pub struct SceneryAudioStats {
    /// Voice entities (all definitions of all placed objects).
    pub voices: usize,
    /// Continuous voices with an emitter now (within range, clip ready).
    pub playing: usize,
    /// Triggered one-shots started.
    pub one_shots: u64,
    /// Triggers that found their clip still loading.
    pub cold_misses: u64,
    /// Definitions that wait for a `[conditionSingle]` (not supported yet).
    pub unsupported: usize,
    /// Definitions whose sound file was not found.
    pub unresolved: usize,
}

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

/// The value of a variable a `[volcurve]` reads: the session clock's, by name. Unknown names read
/// 0 (the format page: "treat input as 0").
#[must_use]
pub fn host_variable(name: &str, host: &HostValues) -> Option<f32> {
    Some(match name.trim().to_lowercase().as_str() {
        "time" => host.time_s,
        "day" => host.day,
        "month" => host.month,
        "year" => host.year,
        "dayofyear" => host.day_of_year,
        "sunalt" => host.sun_alt_deg,
        "temperature" | "weather_temperature" => host.temperature_c,
        "preciprate" => host.precip_rate,
        "preciptype" => host.precip_type,
        _ => return None,
    })
}

/// The level of a definition: the product of its volume curves over the clock.
#[must_use]
pub fn level_of(def: &SoundDef, host: &HostValues) -> f32 {
    level(def, |n| host_variable(n, host).unwrap_or(0.0))
}

type VoiceQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut SceneryVoice,
        &'static GlobalTransform,
        Option<&'static mut SoundEmitter>,
    ),
>;

/// Starts and stops the looping emitters of continuous voices by distance, four times a second,
/// gates them by their conditions and plays the `[noloop]` voices whose conditions just turned
/// true.
#[allow(clippy::too_many_arguments)]
pub fn stream_scenery_voices(
    time: Res<Time>,
    clock: Res<SceneryClock>,
    listener: Query<&GlobalTransform, With<MainCamera>>,
    mut voices: VoiceQuery,
    mut loader: ResMut<ClipLoader>,
    mut stats: ResMut<SceneryAudioStats>,
    mut since: Local<f32>,
    mut commands: Commands,
) {
    *since += time.delta_secs();
    if *since < 0.25 {
        return;
    }
    *since = 0.0;
    let Some(listener) = listener.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    let host = HostValues::of(&clock);
    let read = |n: &str| host_variable(n, &host).unwrap_or(0.0);
    let (mut total, mut playing, mut unsupported, mut unresolved) = (0, 0, 0, 0);
    for (entity, mut voice, tf, emitter) in &mut voices {
        total += 1;
        if voice.playback == Playback::Unsupported {
            unsupported += 1;
            continue;
        }
        let Some(path) = voice.file.clone() else {
            unresolved += 1;
            continue;
        };
        let active = conditions_hold(&voice.def.conditions, read).unwrap_or(false);
        let curve = voice.distance();
        let d = listener.distance(tf.translation());
        match voice.playback {
            Playback::Loop => match emitter {
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
                        if let ClipState::Ready(clip) = loader.state(&path) {
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
            },
            Playback::OnEdge => {
                let edge = active && !voice.was_active;
                if voice.was_active != active {
                    voice.was_active = active;
                }
                if edge && d <= curve.max {
                    match loader.state(&path) {
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
            Playback::OnTrigger | Playback::Unsupported => {}
        }
    }
    stats.voices = total;
    stats.playing = playing;
    stats.unsupported = unsupported;
    stats.unresolved = unresolved;
}

/// Plays the one-shots whose `[trigger]` the objects' scripts raised this frame.
pub fn fire_scenery_triggers(
    clock: Res<SceneryClock>,
    mut objects: Query<(&mut SceneryScripts, &Children)>,
    voices: Query<(&SceneryVoice, &GlobalTransform)>,
    mut loader: ResMut<ClipLoader>,
    mut stats: ResMut<SceneryAudioStats>,
    mut commands: Commands,
) {
    let host = HostValues::of(&clock);
    for (mut scripts, children) in &mut objects {
        // No change flag for the thousands of objects that raised nothing.
        let raised = {
            let s = scripts.bypass_change_detection();
            if !s.has_raised() {
                continue;
            }
            s.take_raised()
        };
        for name in raised {
            for child in children.iter() {
                let Ok((voice, tf)) = voices.get(child) else {
                    continue;
                };
                if voice.playback != Playback::OnTrigger
                    || !voice
                        .def
                        .triggers
                        .iter()
                        .any(|t| t.trim().eq_ignore_ascii_case(&name))
                    || conditions_hold(&voice.def.conditions, |n| {
                        host_variable(n, &host).unwrap_or(0.0)
                    }) != Some(true)
                {
                    continue;
                }
                let Some(path) = voice.file.as_deref() else {
                    continue;
                };
                match loader.state(path) {
                    ClipState::Ready(clip) => {
                        stats.one_shots += 1;
                        commands.spawn((
                            SoundEmitter::new(clip, SoundBus::Ambient)
                                .spatial(voice.distance())
                                .with_speed(voice.def.pitch),
                            Transform::from_translation(tf.translation()),
                            Name::new("scenery one-shot"),
                        ));
                    }
                    // The first ask only starts the decode; the next trigger is heard.
                    ClipState::Loading => stats.cold_misses += 1,
                    ClipState::Failed => {}
                }
            }
        }
    }
}

/// Registers the scenery sound systems.
#[derive(Default)]
pub struct SceneryAudioPlugin;

impl Plugin for SceneryAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClipLoader>()
            .init_resource::<SceneryAudioStats>()
            .add_systems(
                Update,
                (
                    poll_clip_loader,
                    stream_scenery_voices,
                    fire_scenery_triggers,
                )
                    .chain()
                    .after(opts_bevy::scenery_runtime::SceneryRuntimeSet),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_core::formats::sound::SoundConfig;

    fn host(time: f32, day_of_year: f32, sun: f32) -> HostValues {
        let mut h = HostValues::of(&SceneryClock::at(2026, 6, 1, f64::from(time)));
        h.time_s = time;
        h.day_of_year = day_of_year;
        h.sun_alt_deg = sun;
        h
    }

    fn lark() -> SoundDef {
        SoundConfig::parse(
            "[sound]\nlark.wav\n1\n[3d]\n0\n0\n0\n20\n[volcurve]\nDayOfYear\n[pnt]\n29.999\n0\n[pnt]\n30\n1\n[pnt]\n214\n1\n[pnt]\n214.1\n0\n[volcurve]\nSunAlt\n[pnt]\n-5\n0\n[pnt]\n10\n1\n",
        )
        .sounds
        .remove(0)
    }

    #[test]
    fn the_level_follows_the_season_and_the_sun() {
        let d = lark();
        // Summer day, sun high: full. Winter: silent. Summer night: silent. Dawn: half.
        assert!((level_of(&d, &host(43200.0, 100.0, 40.0)) - 1.0).abs() < 1e-5);
        assert_eq!(level_of(&d, &host(43200.0, 10.0, 40.0)), 0.0);
        assert_eq!(level_of(&d, &host(0.0, 100.0, -30.0)), 0.0);
        assert!((level_of(&d, &host(21600.0, 100.0, 2.5)) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn a_voice_fades_from_its_range_to_six_times_that() {
        let v = SceneryVoice {
            def: Arc::new(lark()),
            file: None,
            playback: Playback::Loop,
            was_active: false,
        };
        let c = v.distance();
        assert_eq!((c.reference, c.max), (20.0, 120.0));
        assert_eq!(c.gain(10.0), 1.0);
        assert_eq!(c.gain(121.0), 0.0);
        // Tiny ranges still reach 40 m beyond.
        let mut d = lark();
        d.range = 2.0;
        let v = SceneryVoice {
            def: Arc::new(d),
            file: None,
            playback: Playback::Loop,
            was_active: false,
        };
        assert_eq!(v.distance().max, 42.0);
    }

    #[test]
    fn unknown_variables_read_nothing_and_known_ones_the_clock() {
        let h = host(100.0, 5.0, 7.0);
        assert_eq!(host_variable("TIME", &h), Some(100.0));
        assert_eq!(host_variable("dayofyear", &h), Some(5.0));
        assert_eq!(host_variable("SunAlt", &h), Some(7.0));
        assert_eq!(host_variable("mystery", &h), None);
    }
}
