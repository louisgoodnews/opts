//! `OnExit(InGame)` and quitting: writes the metrics of the drive and gives the global settings back.

use super::super::profile::settings_hash;
use super::super::store::SessionMetrics;
use super::frames::FrameStats;
use super::{ActiveMapProfile, ProfileStoreRes};
use crate::settings::GameSettings;
use crate::settings_registry::SettingsRegistry;
use crate::AppState;
use bevy::app::AppExit;
use bevy::prelude::*;

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
