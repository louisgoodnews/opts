//! The frame of the wizard: header with the stepper, the step body, the persistent summary and the
//! fixed footer; plus the systems that keep that frame current.

use bevy::prelude::*;
use opts_gui::interaction::state::{Disabled, DisabledReason, Selected};
use opts_gui::l10n::Localization;
use opts_gui::patterns::glyphs;
use opts_gui::prelude::*;

use super::planner::{validate, Invalid, PlannerState, PlannerStep};
use super::texts::{what_text, when_text, where_text};
use super::types::{GameCatalogue, NewGameScreenRoot, NewGameSetup};
use super::ui::{action_button, column, glyph, keyed, plain, row, Act, Emphasis, PlannerAction};
use crate::shell::{spawn_screen_root, ShellContent};

mod footer;
mod stepper;
mod summary;

use footer::spawn_footer;
use stepper::spawn_header;
use summary::spawn_summary;

/// The container the step bodies are built into.
#[derive(Component, Debug, Clone, Copy)]
pub struct PlannerBody;

/// A part of the persistent summary (its text).
#[derive(Component, Debug, Clone, Copy)]
pub struct SummaryPart(pub PlannerStep);

/// One button of the stepper.
#[derive(Component, Debug, Clone, Copy)]
pub struct StepperButton {
    /// The step it goes to.
    pub step: PlannerStep,
    /// Node showing the step number.
    pub number: Entity,
    /// Node showing the check of a reached step.
    pub check: Entity,
}

/// Which footer button an entity is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FooterButton {
    /// *Back*.
    Back,
    /// *Next*.
    Next,
    /// *Skip to review*.
    Skip,
    /// *Drive*.
    Drive,
}

/// Spawns the frame (the body is filled by the caller) and returns `(root, body)`.
pub fn spawn_frame(commands: &mut Commands, shell: &ShellContent) -> (Entity, Entity) {
    let root = spawn_screen_root(commands, shell, "PlanADrive");
    commands.entity(root).insert((
        NewGameScreenRoot,
        BackgroundRole(ColorRole::Surface),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(16.0)),
            row_gap: Val::Px(8.0),
            ..column(8.0)
        },
    ));
    let header = spawn_header(commands);
    let body = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                ..column(0.0)
            },
            PlannerBody,
            Name::new("PlannerBody"),
        ))
        .id();
    let summary = spawn_summary(commands);
    let footer = spawn_footer(commands);
    commands
        .entity(root)
        .add_children(&[header, body, summary, footer]);
    (root, body)
}

mod chrome;
pub use chrome::{refresh_chrome, step_invalid};
