//! The profile of a running drive: resolved and applied before the drive loads, the frame time
//! counted while it runs, the global settings restored and the metrics written when it ends
//! (`docs/src/architecture/map-profiles.md` §3.2 and §4.4).

use std::path::PathBuf;

use bevy::app::AppExit;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_core::formats::chrono::{ChronoDate, ChronoSet};

use super::profile::{settings_hash, MapKey, MapProfile};
use super::store::{ProfileSource, ProfileStore, SessionMetrics};
use crate::app::AppArgs;
use crate::in_game::types::SimulationLoadState;
use crate::new_game_screen::planner::setup_date;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::settings::GameSettings;
use crate::settings_registry::SettingsRegistry;
use crate::AppState;

/// Seconds after the load finished that are not counted (shader and tile warm-up).
pub const WARMUP_S: f32 = 10.0;
const BINS: usize = 400;

/// The store the game reads and writes (`OPTS_PROFILES_DIR` overrides the user data folder, for
/// tests and automation); `None` when no data folder can be found.
#[derive(Resource, Debug, Clone)]
pub struct ProfileStoreRes(pub Option<ProfileStore>);

/// The profile the user picked in the map-configuration modal for the next drive: `profile:
/// None` means "no profile, use my global settings". Consumed when the drive starts; ignored when
/// it belongs to another map.
#[derive(Resource, Debug, Clone, Default)]
pub struct MapProfilePick(pub Option<PickedProfile>);

/// See [`MapProfilePick`].
#[derive(Debug, Clone, PartialEq)]
pub struct PickedProfile {
    /// The map the pick is for (folder name).
    pub map: String,
    /// The profile, or none for the global settings.
    pub profile: Option<MapProfile>,
}

/// What is applied to the drive that is running.
#[derive(Resource, Debug, Clone, Default)]
pub struct ActiveMapProfile {
    /// The key of the drive (`None` outside a drive).
    pub key: Option<MapKey>,
    /// Name of the applied profile.
    pub name: Option<String>,
    /// Where it came from (the picked profile counts as [`ProfileSource::Map`]).
    pub source: Option<ProfileSource>,
    /// The keys the profile overrides.
    pub keys: Vec<String>,
    /// The vehicle's display name, for the metrics.
    pub vehicle: String,
}

/// The frame times of the running drive.
#[derive(Resource, Debug, Clone)]
pub struct FrameStats {
    since_loaded_s: f32,
    seconds: f32,
    sum_ms: f64,
    frames: u64,
    bins: Vec<u32>,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self {
            since_loaded_s: 0.0,
            seconds: 0.0,
            sum_ms: 0.0,
            frames: 0,
            bins: vec![0; BINS],
        }
    }
}

impl FrameStats {
    /// Counts one frame of `dt_s` seconds once the warm-up of [`WARMUP_S`] is over.
    pub fn frame(&mut self, dt_s: f32) {
        self.since_loaded_s += dt_s;
        if self.since_loaded_s <= WARMUP_S || !dt_s.is_finite() || dt_s <= 0.0 {
            return;
        }
        let ms = f64::from(dt_s) * 1000.0;
        self.seconds += dt_s;
        self.sum_ms += ms;
        self.frames += 1;
        self.bins[(ms as usize).min(BINS - 1)] += 1;
    }

    /// Average frame time (ms), none before a frame was counted.
    #[must_use]
    pub fn average_ms(&self) -> Option<f32> {
        (self.frames > 0).then(|| (self.sum_ms / self.frames as f64) as f32)
    }

    /// 95th-percentile frame time (ms; 1 ms resolution).
    #[must_use]
    pub fn p95_ms(&self) -> Option<f32> {
        if self.frames == 0 {
            return None;
        }
        let target = (self.frames as f64 * 0.95).ceil() as u64;
        let mut seen = 0u64;
        for (i, n) in self.bins.iter().enumerate() {
            seen += u64::from(*n);
            if seen >= target {
                return Some(i as f32 + 1.0);
            }
        }
        Some(BINS as f32)
    }

    /// Seconds counted (after the warm-up).
    #[must_use]
    pub fn seconds(&self) -> f32 {
        self.seconds
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

/// The folder of the chosen map, from the command line or the OMSI installation.
fn map_dir(
    setup: &NewGameSetup,
    catalogue: Option<&GameCatalogue>,
    args: Option<&AppArgs>,
) -> Option<PathBuf> {
    if let Some(p) = args.and_then(|a| a.0.map_path.clone()) {
        return Some(p);
    }
    let id = catalogue
        .and_then(|c| c.map_by_name(&setup.selected_map))
        .map_or_else(|| setup.selected_map.clone(), |m| m.id.clone());
    Some(crate::omsi_root::known_omsi_root()?.join("maps").join(id))
}

/// The Chrono entries active on the date of the plan, as their paths below `Chrono/`, in the
/// order of the set (the newest last).
#[must_use]
pub fn active_chrono(map_dir: &std::path::Path, setup: &NewGameSetup) -> Vec<String> {
    let set = ChronoSet::scan(map_dir);
    let d = setup_date(setup);
    let active = set.active(ChronoDate::from_ymd(d.year, d.month, d.day));
    active
        .indices()
        .iter()
        .filter_map(|i| set.entries().get(*i).map(|e| e.rel.clone()))
        .collect()
}

/// The key of the plan: the map folder and the Chrono entries active on its date.
#[must_use]
pub fn key_of(
    setup: &NewGameSetup,
    catalogue: Option<&GameCatalogue>,
    args: Option<&AppArgs>,
) -> Option<MapKey> {
    let dir = map_dir(setup, catalogue, args)?;
    let name = dir.file_name()?.to_string_lossy().into_owned();
    // Several entries can be active at once (Berlin-Spandau: fifteen cumulative ones on a 1990s
    // date); the newest, the last in the order of the set, names the state of the map.
    let active = active_chrono(&dir, setup);
    let newest: Vec<&str> = active.last().map(String::as_str).into_iter().collect();
    Some(MapKey::new(&name, &newest))
}

/// What names the map of the drive: the setup, the catalogue and the command line.
#[derive(SystemParam)]
pub struct DriveChoice<'w> {
    /// The choices of the new-game wizard.
    pub setup: Res<'w, NewGameSetup>,
    /// The maps and vehicles read.
    pub catalogue: Option<Res<'w, GameCatalogue>>,
    /// The command line.
    pub args: Option<Res<'w, AppArgs>>,
}

/// `OnEnter(InGame)`, before the session is set up: resolves the profile (the picked one, else
/// the fallback chain) and applies it to the session's settings.
pub fn apply_on_enter(
    chosen: DriveChoice,
    registry: Res<SettingsRegistry>,
    store: Res<ProfileStoreRes>,
    mut pick: ResMut<MapProfilePick>,
    mut settings: ResMut<GameSettings>,
    mut active: ResMut<ActiveMapProfile>,
    mut stats: ResMut<FrameStats>,
) {
    let DriveChoice {
        setup,
        catalogue,
        args,
    } = chosen;
    stats.reset();
    *active = ActiveMapProfile::default();
    // A drive that starts while another profile is still on (a restart without leaving) first
    // gives the global values back.
    settings.restore_session();
    let Some(key) = key_of(&setup, catalogue.as_deref(), args.as_deref()) else {
        return;
    };
    let picked = pick.0.take().filter(|p| slug_eq(&p.map, &key.map_id));
    let (profile, source) = match picked {
        Some(p) => (p.profile, ProfileSource::Map),
        None => match store.0.as_ref().map(|s| s.resolve(&registry, &key)) {
            Some(r) => {
                for d in &r.dropped {
                    tracing::warn!(key = %d.key, reason = %d.reason, "map profile: setting left out");
                }
                (r.profile, r.source)
            }
            None => (None, ProfileSource::Global),
        },
    };
    active.vehicle = if setup.no_vehicle {
        String::new()
    } else {
        setup.selected_vehicle.clone()
    };
    active.key = Some(key.clone());
    let Some(profile) = profile else {
        tracing::info!(map = %key.map_id, chrono = %key.chrono_id, "no map profile: global settings");
        return;
    };
    settings.overlay_session(&profile.settings);
    active.name = Some(profile.name.clone());
    active.source = Some(source);
    active.keys = profile.settings.keys().cloned().collect();
    tracing::info!(
        map = %key.map_id,
        chrono = %key.chrono_id,
        profile = %profile.name,
        keys = active.keys.len(),
        "map profile applied for this drive"
    );
}

fn slug_eq(map: &str, id: &str) -> bool {
    super::profile::slug(map) == id
}

/// Counts the frame time of the drive.
pub fn track_frame_time(
    time: Res<Time<Real>>,
    virtual_time: Res<Time<Virtual>>,
    load: Option<Res<SimulationLoadState>>,
    mut stats: ResMut<FrameStats>,
) {
    if !load.is_some_and(|l| l.is_complete) || virtual_time.is_paused() {
        return;
    }
    stats.frame(time.delta_secs());
}

/// The metrics of the drive that is ending, if long enough to count.
fn session_metrics(
    active: &ActiveMapProfile,
    stats: &FrameStats,
    settings: &GameSettings,
    registry: &SettingsRegistry,
) -> Option<(String, SessionMetrics)> {
    let key = active.key.as_ref()?;
    let (avg, p95) = (stats.average_ms()?, stats.p95_ms()?);
    let values = registry
        .profile_keys()
        .into_iter()
        .filter_map(|k| settings.raw(k).map(|v| (k.to_string(), v)))
        .collect();
    Some((
        key.map_id.clone(),
        SessionMetrics {
            chrono: key.chrono_id.clone(),
            avg_ms: avg,
            p95_ms: p95,
            seconds: stats.seconds(),
            vehicle: active.vehicle.clone(),
            settings_hash: settings_hash(&values),
            date: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        },
    ))
}

fn finish(
    active: &mut ActiveMapProfile,
    stats: &mut FrameStats,
    settings: &mut GameSettings,
    registry: &SettingsRegistry,
    store: &ProfileStoreRes,
) {
    if let (Some(store), Some((map, m))) = (
        store.0.clone(),
        session_metrics(active, stats, settings, registry),
    ) {
        // Written off the main thread; losing it costs one session of advice.
        let _ = std::thread::Builder::new()
            .name("map-metrics".into())
            .spawn(move || {
                if let Err(e) = store.record_session(&map, m) {
                    tracing::warn!("could not record the map metrics: {e}");
                }
            });
    }
    settings.restore_session();
    *active = ActiveMapProfile::default();
    stats.reset();
}

/// `OnExit(InGame)`: writes the metrics and gives the global settings back.
pub fn finish_on_exit(
    registry: Res<SettingsRegistry>,
    store: Res<ProfileStoreRes>,
    mut settings: ResMut<GameSettings>,
    mut active: ResMut<ActiveMapProfile>,
    mut stats: ResMut<FrameStats>,
) {
    finish(&mut active, &mut stats, &mut settings, &registry, &store);
}

/// The application is quitting during a drive: the metrics still count.
pub fn finish_on_quit(
    mut exits: MessageReader<AppExit>,
    state: Option<Res<State<AppState>>>,
    registry: Res<SettingsRegistry>,
    store: Res<ProfileStoreRes>,
    mut settings: ResMut<GameSettings>,
    mut active: ResMut<ActiveMapProfile>,
    mut stats: ResMut<FrameStats>,
) {
    if exits.read().next().is_none() || !state.is_some_and(|s| *s.get() == AppState::InGame) {
        return;
    }
    finish(&mut active, &mut stats, &mut settings, &registry, &store);
}
