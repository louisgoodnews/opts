//! Session save and restore (`docs/src/architecture/session-save.md`, M-0069, ADR-0011).
//!
//! * [`container`]: the file format (magic, versions, metadata, fingerprint, sections). Pure data.
//! * [`store`]: the folder of saves; atomic writes, backup, recovery. Pure file handling.
//! * [`registry`]: [`SaveSection`]s that feature code registers; the save system calls them and
//!   never looks inside.
//! * [`sections`]: the sections this build saves (session, time, weather, vehicles, active vehicle,
//!   camera, counters, simulation).
//! * [`ops`] and [`restore`]: saving the running session and restoring a save.
//! * [`ui`]: the *Save game* and *Load game* overlay; [`quick`]: quick save, quick load, autosave.

pub mod container;
pub mod fingerprint;
pub mod ops;
pub mod quick;
pub mod registry;
pub mod restore;
pub mod sections;
mod sections_runtime;
pub mod store;
pub mod ui;

use bevy::prelude::*;

pub use container::{SaveError, SaveFile, SaveMeta, SectionBlob};
pub use ops::{save_session, SaveRequest, Saved};
pub use registry::{SaveAppExt, SaveRegistry, SaveSection, Stage};
pub use restore::{begin_restore, LastRestore, PendingRestore, RestoreError, RestoreOutcome};
pub use store::SaveStore;

use crate::app_state::AppState;
use crate::in_game::types::SimulationLoadState;

/// The folder the saves live in (`None`: no data directory is known, saving is unavailable).
/// Tests and the control protocol's headless runs replace it.
#[derive(Resource, Debug, Clone)]
pub struct SaveStorage(pub Option<SaveStore>);

impl Default for SaveStorage {
    fn default() -> Self {
        Self(SaveStore::user())
    }
}

/// Seconds the current drive has been played, across loads of the same save (real time while a
/// loaded session is up, paused or not).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayClock {
    /// The played time in seconds.
    pub played_s: f64,
}

fn tick_play_clock(
    time: Res<Time<Real>>,
    load: Res<SimulationLoadState>,
    mut clock: ResMut<PlayClock>,
) {
    if load.is_complete {
        clock.played_s += f64::from(time.delta_secs());
    }
}

fn reset_play_clock(mut clock: ResMut<PlayClock>, pending: Option<Res<PendingRestore>>) {
    // A restore has set the clock to the saved value before the session is entered again.
    if pending.is_none() {
        *clock = PlayClock::default();
    }
}

/// Registers the save system, the built-in sections, the quick keys and the autosave.
#[derive(Default)]
pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveStorage>()
            .init_resource::<SaveRegistry>()
            .init_resource::<PlayClock>()
            .init_resource::<LastRestore>();
        sections::register(app);
        app.add_plugins(ui::SaveUiPlugin)
            .add_plugins(quick::QuickSavePlugin)
            .add_systems(
                Update,
                (tick_play_clock, restore::drive_restore)
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnExit(AppState::InGame), reset_play_clock)
            .add_systems(OnEnter(AppState::MainMenu), restore::forget_pending);
    }
}
