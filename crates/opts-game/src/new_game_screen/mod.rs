//! *Plan a drive*: the four-step wizard that replaced "Start New Game".
//!
//! The steps are *Where* (map), *What* (vehicle, livery, equipment), *When* (date, time, weather),
//! *Trip* (a timetable run, only when the plan has runs to offer) and *Review*; the first two are built on the master-detail pattern of
//! [`opts_gui::patterns`]. The specification is
//! `docs/src/roadmap/phase-9/wizard-and-loading.md`; the choices live in [`NewGameSetup`] and are
//! remembered through [`crate::drive_history`].

pub mod choice;
pub mod configs;
pub mod formation;
pub mod history;
pub mod livery;
pub mod options;
pub mod planner;
pub mod repaints;
pub mod setvars_discovery;
pub mod step_load;
pub mod step_review;
pub mod step_what;
pub mod step_when;
pub mod step_where;
pub mod thumbnails;
pub mod trips;
pub mod types;
pub mod weather;

mod screen;
mod systems;
pub(crate) mod texts;
pub(crate) mod ui;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_gui::interaction::focus::FocusEvent;
use opts_gui::l10n::Localization;

use crate::app_state::AppState;
use crate::shell::ShellContent;

pub use formation::apply_to_start as apply_formation_at_start;
pub use history::PlannerHistory;
pub use planner::{Note, PlannerState, PlannerStep};
pub use screen::{FooterButton, PlannerBody};
pub use types::{GameCatalogue, MapItem, NewGameScreenRoot, NewGameSetup, VehicleItem};

/// Everything a step needs to build its body.
pub struct BuildCtx<'a> {
    /// Texts.
    pub loc: &'a Localization,
    /// Maps and vehicles.
    pub catalogue: &'a GameCatalogue,
    /// Weather presets.
    pub weather: &'a opts_bevy::WeatherCatalogue,
    /// The choices so far.
    pub setup: &'a NewGameSetup,
    /// Recents and favourites.
    pub history: &'a PlannerHistory,
    /// Remarks about remembered choices that vanished.
    pub notes: &'a [Note],
    /// The timetable runs offered.
    pub trips: &'a trips::TripState,
}

/// What a step is built from (read-only).
#[derive(SystemParam)]
struct WizardRead<'w> {
    loc: Res<'w, Localization>,
    catalogue: Res<'w, GameCatalogue>,
    weather: Res<'w, opts_bevy::WeatherCatalogue>,
    trips: Res<'w, trips::TripState>,
}

/// The texts and the catalogue, for the systems that only need those two.
#[derive(SystemParam)]
pub struct TextsAndCatalogue<'w> {
    /// The texts.
    pub loc: Res<'w, Localization>,
    /// The maps and vehicles read.
    pub catalogue: Res<'w, GameCatalogue>,
}

mod wizard;
use wizard::*;

/// Plugin of the wizard.
#[derive(Default)]
pub struct NewGameMenuPlugin;

impl Plugin for NewGameMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameCatalogue>()
            .init_resource::<opts_bevy::WeatherCatalogue>()
            .init_resource::<NewGameSetup>()
            .init_resource::<PlannerState>()
            .init_resource::<livery::LiveryState>()
            .init_resource::<trips::TripState>()
            .init_resource::<crate::shell::esc::ScreenBackSteps>()
            .add_message::<crate::shell::esc::ScreenBack>()
            .add_message::<crate::shell::nav::NavigateBack>()
            .add_message::<crate::shell::nav::Navigate>()
            .init_resource::<thumbnails::ThumbnailService>();
        if !app.world().contains_resource::<ShellContent>() {
            app.init_resource::<ShellContent>();
        }
        if !app.world().contains_resource::<PlannerHistory>() {
            app.init_resource::<PlannerHistory>();
        }
        if !app.world().contains_resource::<configs::ConfigLibrary>() {
            app.init_resource::<configs::ConfigLibrary>();
        }
        step_load::register(app);
        let active = in_state(AppState::VehicleSelection);
        app.add_systems(OnEnter(AppState::VehicleSelection), enter_wizard)
            .add_systems(OnExit(AppState::VehicleSelection), cleanup_wizard)
            .add_systems(
                Update,
                (
                    systems::handle_actions,
                    systems::handle_tab_actions,
                    systems::handle_start_point,
                    systems::handle_list_events,
                    options::apply_switches,
                    options::apply_choices,
                    options::apply_numbers,
                    configs::track_name,
                    configs::finish_load,
                    options::close_on_cancel,
                )
                    .run_if(active.clone())
                    .run_if(opts_bevy::reporting::not_modal_active)
                    .after(opts_gui::interaction::activation::ActivationSet),
            )
            .add_systems(
                Update,
                systems::handle_back_action
                    .run_if(active.clone())
                    .run_if(opts_bevy::reporting::not_modal_active)
                    .after(crate::shell::esc::shell_back_system),
            )
            .add_systems(
                PreUpdate,
                systems::publish_back_steps.run_if(active.clone()),
            )
            .add_systems(OnExit(AppState::VehicleSelection), reset_back_steps)
            .add_systems(
                Update,
                (
                    trips::refresh_trips,
                    rebuild_body,
                    screen::refresh_chrome,
                    choice::sync_choices,
                    step_when::refresh_when,
                    livery::poll_liveries,
                    thumbnails::fill_thumbnails,
                )
                    .chain()
                    .run_if(active)
                    // After every handler that clears or fills a panel: `poll_liveries` fills the
                    // boxes it finds in the detail panel, so the commands that empty the panel
                    // must be applied first (an unordered pair raced, M-0078).
                    .after(systems::handle_actions)
                    .after(systems::handle_tab_actions)
                    .after(systems::handle_list_events),
            );
    }
}
