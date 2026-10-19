//! Per-map configuration profiles and the load profiler
//! (`docs/src/architecture/map-profiles.md`, [M-0130](../../../../docs/src/roadmap/phase-9/M-0130.md)).
//!
//! * [`profile`]: the profile, its key (map and Chrono entry), presets and TOML text.
//! * [`store`]: the files under `<data dir>/profiles/`, the fallback chain, the metrics file.
//! * [`advice`]: [`advice::MapLoadProfile`] and the heuristic engine.

pub mod advice;
pub mod modal;
pub mod profile;
pub mod service;
pub mod session;
pub mod static_profile;
pub mod store;

pub use advice::{advise, Advice, AdviceId, MapLoadProfile, Severity};
pub use profile::{presets, slug, MapKey, MapProfile, DEFAULT_CHRONO};
pub use store::{ProfileSource, ProfileStore, Resolved, SessionMetrics};

#[cfg(test)]
mod tests;

/// Registers the profile resources and the drive hooks.
#[derive(Default)]
pub struct MapProfilesPlugin;

impl bevy::prelude::Plugin for MapProfilesPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::*;
        let store = std::env::var_os("OPTS_PROFILES_DIR")
            .map(ProfileStore::new)
            .or_else(ProfileStore::user);
        if let Some(mut loc) = app
            .world_mut()
            .get_resource_mut::<opts_gui::l10n::Localization>()
        {
            loc.extend("en", include_str!("messages_en.ftl"));
            loc.extend("de", include_str!("messages_de.ftl"));
        }
        app.insert_resource(session::ProfileStoreRes(store))
            .init_resource::<session::MapProfilePick>()
            .init_resource::<session::ActiveMapProfile>()
            .init_resource::<session::FrameStats>()
            .init_resource::<static_profile::StaticProfiles>()
            .init_resource::<modal::MapConfigMenu>()
            .add_systems(
                OnEnter(crate::AppState::InGame),
                session::apply_on_enter.before(crate::in_game::setup_in_game_session),
            )
            .add_systems(OnExit(crate::AppState::InGame), session::finish_on_exit)
            .add_systems(
                Update,
                session::track_frame_time.run_if(in_state(crate::AppState::InGame)),
            )
            .add_systems(Update, static_profile::poll_static_profiles)
            .add_systems(
                Update,
                (
                    modal::refresh_view,
                    modal::track_name,
                    modal::actions,
                    modal::close_on_cancel,
                    modal::rebuild,
                )
                    .chain()
                    .run_if(in_state(crate::AppState::VehicleSelection)),
            )
            .add_systems(Last, session::finish_on_quit);
    }
}
