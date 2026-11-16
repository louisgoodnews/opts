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

mod actions;
mod view;
pub use actions::{apply, create_from_current, delete};
pub use view::view;
