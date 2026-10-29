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

/// Entering the wizard: first step, the last drive's choices (once per session), the frame.
#[allow(clippy::too_many_arguments)]
fn enter_wizard(
    mut commands: Commands,
    shell: Res<ShellContent>,
    loc: Res<Localization>,
    catalogue: Res<GameCatalogue>,
    weather: Res<opts_bevy::WeatherCatalogue>,
    mut setup: ResMut<NewGameSetup>,
    mut history: ResMut<PlannerHistory>,
    mut state: ResMut<PlannerState>,
    trips: Res<trips::TripState>,
    mut events: MessageWriter<FocusEvent>,
) {
    let notes = history.apply_last(&mut setup, &catalogue);
    // A remembered drive brings its date and time: choosing a map keeps them.
    if history.history.last().is_some() && !catalogue.maps.is_empty() {
        state.date_chosen = true;
        state.time_chosen = true;
    }
    planner::ensure_valid(&mut setup, &catalogue);
    state.step = PlannerStep::Where;
    state.built = Some(PlannerStep::Where);
    state.notes = notes;
    let (_, body) = screen::spawn_frame(&mut commands, &shell);
    let ctx = BuildCtx {
        loc: &loc,
        catalogue: &catalogue,
        weather: &weather,
        setup: &setup,
        history: &history,
        notes: &state.notes,
        trips: &trips,
    };
    let built = build_step(&mut commands, &ctx, state.step);
    commands.entity(body).add_child(built.0);
    if let Some(f) = built.1 {
        events.write(FocusEvent::Set(f));
    }
}

/// Builds the body of `step`; returns its root and the control that should take focus.
fn build_step(
    commands: &mut Commands,
    ctx: &BuildCtx,
    step: PlannerStep,
) -> (Entity, Option<Entity>) {
    match step {
        PlannerStep::Where => {
            let root = step_where::spawn_where(commands, ctx);
            (root, None)
        }
        PlannerStep::What => {
            let root = step_what::spawn_what(commands, ctx);
            (root, None)
        }
        PlannerStep::When => (step_when::spawn_when(commands, ctx), None),
        PlannerStep::Trip => (trips::spawn_trip(commands, ctx), None),
        PlannerStep::Review => (step_review::spawn_review(commands, ctx), None),
    }
}

/// A step change replaces the body.
#[allow(clippy::too_many_arguments)]
fn rebuild_body(
    mut state: ResMut<PlannerState>,
    body: Query<Entity, With<PlannerBody>>,
    loc: Res<Localization>,
    catalogue: Res<GameCatalogue>,
    weather: Res<opts_bevy::WeatherCatalogue>,
    setup: Res<NewGameSetup>,
    history: Res<PlannerHistory>,
    trips: Res<trips::TripState>,
    mut commands: Commands,
    mut built_with: Local<(usize, usize, usize)>,
) {
    let Ok(body) = body.single() else {
        return;
    };
    // The catalogue may be (re)read while the wizard is open (the first scan of a fresh
    // installation takes seconds): a step built from an empty or older list is built again.
    let content = (
        catalogue.maps.len(),
        catalogue.vehicles.len(),
        weather.ids().len(),
    );
    let content_changed = *built_with != content;
    *built_with = content;
    if state.built == Some(state.step) && !content_changed {
        return;
    }
    state.built = Some(state.step);
    commands.entity(body).despawn_related::<Children>();
    let ctx = BuildCtx {
        loc: &loc,
        catalogue: &catalogue,
        weather: &weather,
        setup: &setup,
        history: &history,
        notes: &state.notes,
        trips: &trips,
    };
    let (root, _) = build_step(&mut commands, &ctx, state.step);
    commands.entity(body).add_child(root);
}

/// Leaving the wizard removes its screen and dialogs.
fn cleanup_wizard(
    mut commands: Commands,
    roots: Query<Entity, With<NewGameScreenRoot>>,
    modals: Query<Entity, With<options::OptionsModal>>,
) {
    for e in roots.iter().chain(modals.iter()) {
        commands.entity(e).despawn();
    }
}

/// Leaving the wizard: *Back* on the next screen has no step of this one to take.
fn reset_back_steps(mut steps: ResMut<crate::shell::esc::ScreenBackSteps>) {
    steps.0 = false;
}
