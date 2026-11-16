//! `OnEnter(InGame)`: applies the profile of the drive to the session settings before it loads.

use super::super::store::ProfileSource;
use super::frames::FrameStats;
use super::key::{key_of, DriveChoice};
use super::{ActiveMapProfile, MapProfilePick, ProfileStoreRes};
use crate::settings::GameSettings;
use crate::settings_registry::SettingsRegistry;
use bevy::prelude::*;

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
    super::super::profile::slug(map) == id
}
