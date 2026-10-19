//! What the map-configuration modal and the control protocol share: the view of the chosen map
//! (resolved profile, saved profiles, presets, load profile, advice) and the actions on it.

use bevy::prelude::*;

use super::advice::{advise, Advice, MapLoadProfile};
use super::profile::{presets, MapKey, MapProfile, DEFAULT_CHRONO};
use super::session::{active_chrono, key_of, MapProfilePick, PickedProfile, ProfileStoreRes};
use super::static_profile::{request_for, StaticProfiles};
use super::store::{ProfileSource, Resolved};
use crate::app::AppArgs;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::settings::GameSettings;
use crate::settings_registry::SettingsRegistry;

/// Everything the modal shows about the chosen map.
#[derive(Debug, Clone)]
pub struct ProfileView {
    /// Map folder name.
    pub map_dir_name: String,
    /// The key (map and Chrono entry) of the plan.
    pub key: MapKey,
    /// The Chrono entries active on the plan's date (paths below `Chrono/`).
    pub chrono_entries: Vec<String>,
    /// What the fallback chain gives.
    pub resolved: Resolved,
    /// The profile files of this map: `(chrono_id, profile)`.
    pub saved: Vec<(String, MapProfile)>,
    /// The built-in presets.
    pub presets: Vec<MapProfile>,
    /// The load profile (counts, last session).
    pub load: MapLoadProfile,
    /// The counts are still being worked on (a sample or nothing yet).
    pub counting: bool,
    /// Advice for the load profile.
    pub advice: Vec<Advice>,
    /// The profile the user picked for the next drive: `Some(None)` is "global settings".
    pub picked: Option<Option<String>>,
}

/// Why an action could not be done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    /// No map is chosen or the installation is not found.
    NoMap,
    /// No data folder for the profiles.
    NoStore,
    /// No such profile.
    NotFound(String),
    /// The file could not be written or removed.
    Io(String),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoMap => f.write_str("no map is chosen"),
            Self::NoStore => f.write_str("there is no folder for profiles"),
            Self::NotFound(n) => write!(f, "no profile named \"{n}\""),
            Self::Io(e) => write!(f, "the profile file could not be changed: {e}"),
        }
    }
}

fn map_context(world: &World) -> Option<(MapKey, String, std::path::PathBuf, Vec<String>)> {
    let setup = world.get_resource::<NewGameSetup>()?;
    let catalogue = world.get_resource::<GameCatalogue>();
    let args = world.get_resource::<AppArgs>();
    let key = key_of(setup, catalogue, args)?;
    let dir = args.and_then(|a| a.0.map_path.clone()).or_else(|| {
        let id = catalogue
            .and_then(|c| c.map_by_name(&setup.selected_map))
            .map_or_else(|| setup.selected_map.clone(), |m| m.id.clone());
        Some(crate::omsi_root::known_omsi_root()?.join("maps").join(id))
    })?;
    let name = dir.file_name()?.to_string_lossy().into_owned();
    let chrono = active_chrono(&dir, setup);
    Some((key, name, dir, chrono))
}

/// The view of the plan's map; starts the static count the first time.
pub fn view(world: &mut World) -> Option<ProfileView> {
    let (key, name, dir, chrono_entries) = map_context(world)?;
    let store = world.resource::<ProfileStoreRes>().clone();
    world.resource_scope(|world, mut statics: Mut<StaticProfiles>| {
        request_for(&mut statics, &store, &key.map_id, dir);
        let registry = world.resource::<SettingsRegistry>();
        let (resolved, saved, last) = match &store.0 {
            Some(s) => (
                s.resolve(registry, &key),
                s.list_map(registry, &key.map_id),
                s.last_session(&key),
            ),
            None => (
                Resolved {
                    profile: None,
                    source: ProfileSource::Global,
                    dropped: Vec::new(),
                },
                Vec::new(),
                None,
            ),
        };
        let stat = statics.get(&key.map_id);
        let load = stat.map_or_else(MapLoadProfile::default, |s| {
            s.to_load_profile(
                last.as_ref().map(|m| m.avg_ms),
                last.as_ref().map(|m| m.p95_ms),
            )
        });
        let traffic = world
            .resource::<GameSettings>()
            .raw("gameplay.traffic")
            .and_then(|v| v.parse().ok())
            .unwrap_or(100);
        let advice = if stat.is_some() {
            advise(&load, traffic)
        } else {
            Vec::new()
        };
        let picked = world
            .resource::<MapProfilePick>()
            .0
            .as_ref()
            .filter(|p| super::profile::slug(&p.map) == key.map_id)
            .map(|p| p.profile.as_ref().map(|x| x.name.clone()));
        Some(ProfileView {
            map_dir_name: name,
            counting: stat.is_none_or(|s| s.estimated),
            key,
            chrono_entries,
            resolved,
            saved,
            presets: presets(),
            load,
            advice,
            picked,
        })
    })
}

/// A profile to use: a preset, one of the map's files, or none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    /// No profile: the global settings.
    Global,
    /// A built-in preset by id (`performance`, `balanced`, `quality`) or name.
    Preset(String),
    /// A saved profile of this map by name.
    Saved(String),
}

fn find(view: &ProfileView, choice: &Choice) -> Result<Option<MapProfile>, ServiceError> {
    match choice {
        Choice::Global => Ok(None),
        Choice::Preset(n) => view
            .presets
            .iter()
            .find(|p| {
                p.based_on
                    .as_deref()
                    .is_some_and(|b| b.eq_ignore_ascii_case(n))
                    || p.name.eq_ignore_ascii_case(n)
            })
            .cloned()
            .map(Some)
            .ok_or_else(|| ServiceError::NotFound(n.clone())),
        Choice::Saved(n) => view
            .saved
            .iter()
            .find(|(c, p)| p.name.eq_ignore_ascii_case(n) || c.eq_ignore_ascii_case(n))
            .map(|(_, p)| Some(p.clone()))
            .ok_or_else(|| ServiceError::NotFound(n.clone())),
    }
}

/// *Apply Selected*: the choice is used for the next drive of this map; with `make_default` it is
/// also written as the profile of this key (`default.toml` or the Chrono entry's file).
///
/// # Errors
/// No map, no such profile, or the file cannot be written.
pub fn apply(world: &mut World, choice: &Choice, make_default: bool) -> Result<(), ServiceError> {
    let v = view(world).ok_or(ServiceError::NoMap)?;
    let profile = find(&v, choice)?;
    if make_default {
        if let Some(p) = &profile {
            let store = world
                .resource::<ProfileStoreRes>()
                .0
                .clone()
                .ok_or(ServiceError::NoStore)?;
            store
                .save(&v.key, p)
                .map_err(|e| ServiceError::Io(e.to_string()))?;
        }
    }
    world.resource_mut::<MapProfilePick>().0 = Some(PickedProfile {
        map: v.map_dir_name,
        profile,
    });
    Ok(())
}

/// *Create New from Current*: a profile of the current global values of the map-eligible
/// settings, saved for the key (`for_chrono` false: the map's default). Returns its name.
///
/// # Errors
/// No map, no folder, or the file cannot be written.
pub fn create_from_current(
    world: &mut World,
    name: &str,
    for_chrono: bool,
) -> Result<String, ServiceError> {
    let v = view(world).ok_or(ServiceError::NoMap)?;
    let store = world
        .resource::<ProfileStoreRes>()
        .0
        .clone()
        .ok_or(ServiceError::NoStore)?;
    let profile = {
        let registry = world.resource::<SettingsRegistry>();
        let settings = world.resource::<GameSettings>();
        // A key nothing is stored for holds its default: the profile pins the value the
        // person sees now, not "whatever the global setting is later".
        MapProfile::from_current(name, registry, |k| {
            settings
                .raw(k)
                .or_else(|| registry.get(k).map(|d| d.default.to_string()))
        })
    };
    let key = if for_chrono && v.key.chrono_id != DEFAULT_CHRONO {
        v.key.clone()
    } else {
        v.key.map_default()
    };
    store
        .save(&key, &profile)
        .map_err(|e| ServiceError::Io(e.to_string()))?;
    Ok(profile.name)
}

/// *Delete*: removes the profile file of the key (the Chrono one when `for_chrono` and one is
/// active, else the map's default). Returns whether there was one.
///
/// # Errors
/// No map, no folder, or the file cannot be removed.
pub fn delete(world: &mut World, for_chrono: bool) -> Result<bool, ServiceError> {
    let v = view(world).ok_or(ServiceError::NoMap)?;
    let store = world
        .resource::<ProfileStoreRes>()
        .0
        .clone()
        .ok_or(ServiceError::NoStore)?;
    let key = if for_chrono && v.key.chrono_id != DEFAULT_CHRONO {
        v.key.clone()
    } else {
        v.key.map_default()
    };
    store
        .delete(&key)
        .map_err(|e| ServiceError::Io(e.to_string()))
}
