//! The profile of a running drive: resolved and applied before the drive loads, the frame time
//! counted while it runs, the global settings restored and the metrics written when it ends
//! (`docs/src/architecture/map-profiles.md` §3.2 and §4.4).

use bevy::prelude::*;
use super::profile::{MapKey, MapProfile};
use super::store::{ProfileSource, ProfileStore};

mod apply;
mod finish;
mod frames;
mod key;
pub use apply::apply_on_enter;
pub use finish::{finish_on_exit, finish_on_quit};
pub use frames::{track_frame_time, FrameStats, WARMUP_S};
pub use key::{active_chrono, key_of, DriveChoice};

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
