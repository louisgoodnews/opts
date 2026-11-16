//! The pause menu's *Environment* page: date, time of day and weather of the running drive.
//!
//! Time and weather change at once while the page is open (the sky and the light follow behind
//! the overlay); the date is a draft that applies with *Apply* (it may change the map's content,
//! Chrono). *Cancel* and Esc restore the time and weather the page opened with. All changes go
//! through [`EnvironmentChange`] (`docs/src/architecture/session-environment.md`).

use bevy::prelude::*;
use opts_bevy::{
    CalendarDate, ChronoRebuild, EnvironmentChange, MapDatePolicy, SessionEnvironment,
    WeatherCatalogue,
};
use opts_gui::components::modal::{ModalBuilder, ModalEvent, ModalRoot};
use opts_gui::interaction::activation::Activated;
use opts_gui::l10n::{L10nArgs, L10nKey, Localization};
use opts_gui::prelude::*;

use crate::new_game_screen::weather::{presets, Label};
use crate::shell::calendar::{self, ChoiceMark, Field};
use crate::shell::keys;

/// Marks the page's modal root.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct EnvironmentOverlay;

/// What a control of the page does.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvAction {
    /// Change a date (draft) or time (live) field by `delta`.
    Step(Field, i32),
    /// Choose a weather preset (live); the id is the control's [`WeatherId`].
    Weather,
    /// Switch the lighting quality between automatic and custom (`graphics.light_quality_mode`).
    LightMode,
    /// Apply the date and close.
    Apply,
    /// Restore time and weather and close.
    Cancel,
}

/// The preset id of a weather chip.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct WeatherId(pub String);

/// A value text of a stepper.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvValue(pub Field);

/// The line that reports a running map rebuild.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct RebuildLine;

/// State of the page.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct EnvironmentPage {
    /// The page is open.
    pub open: bool,
    /// Time of day and weather when the page opened (restored by *Cancel*).
    pub snapshot: Option<(f64, String)>,
    /// The date being chosen (applied by *Apply*).
    pub draft_date: CalendarDate,
}

/// Opens the page (the caller has put the pause overlay aside).
pub fn open_environment_page(commands: &mut Commands) {
    commands.queue(|world: &mut World| {
        if world
            .query_filtered::<(), With<EnvironmentOverlay>>()
            .iter(world)
            .next()
            .is_some()
        {
            return;
        }
        let env = world
            .get_resource::<SessionEnvironment>()
            .cloned()
            .unwrap_or_default();
        *world.resource_mut::<EnvironmentPage>() = EnvironmentPage {
            open: true,
            snapshot: Some((env.seconds_of_day(), env.weather().to_owned())),
            draft_date: env.date(),
        };
        let weather = world
            .get_resource::<WeatherCatalogue>()
            .cloned()
            .unwrap_or_default();
        let loc = world
            .get_resource::<Localization>()
            .cloned()
            .unwrap_or_else(|| Localization::new("en"));
        let mut commands = world.commands();
        spawn_page(&mut commands, &weather, &loc);
        world.flush();
    });
}

mod layout;
mod spawn;
use spawn::spawn_page;

mod params;
mod systems;
pub(crate) use systems::resume_drive;
pub use systems::{build, environment_page_input, refresh_environment_page, resume_after_rebuild};
