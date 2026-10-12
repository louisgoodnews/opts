//! Small builders shared by the steps: keyed texts, rows and columns, action buttons.

use bevy::prelude::*;
use opts_gui::icons::icon_text;
use opts_gui::l10n::{L10nArgs, L10nKey};
use opts_gui::prelude::*;
use opts_gui::ui_tree::A11yHidden;

use super::planner::{Field, PlannerStep};

/// What a button of the wizard does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlannerAction {
    /// One step back, or out of the wizard from the first step.
    Back,
    /// One step forward.
    Next,
    /// Jump to the review.
    Skip,
    /// Start the drive.
    Drive,
    /// Go to a step (stepper, summary parts, *Change* links).
    GoStep(PlannerStep),
    /// Set the time of day from quick chip `n`.
    Quick(usize),
    /// Choose a weather preset by id.
    Weather(&'static str),
    /// Change a date or time field by `delta` steps.
    Step(Field, i32),
    /// Choose the livery with this name.
    Livery(String),
    /// Open the equipment dialog.
    OpenOptions,
    /// Reset the equipment options to the livery's defaults.
    ResetOptions,
    /// Close the equipment dialog.
    CloseOptions,
    /// Open the settings screen (empty catalogue help).
    OpenSettings,
}

/// The action of a button.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Act(pub PlannerAction);

/// A text that shows message `key`.
pub fn keyed(key: &'static str, style: TypeScale, role: ColorRole) -> impl Bundle {
    (
        Text::new(key),
        L10nKey(key),
        TextColor::default(),
        TypeStyleToken(style),
        ForegroundRole(role),
    )
}

/// A text that shows message `key` with variables.
pub fn keyed_args(
    key: &'static str,
    args: L10nArgs,
    style: TypeScale,
    role: ColorRole,
) -> impl Bundle {
    (keyed(key, style, role), args)
}

/// A text with content from the data (not copy).
pub fn plain(text: impl Into<String>, style: TypeScale, role: ColorRole) -> impl Bundle {
    (
        Text::new(text),
        TextColor::default(),
        TypeStyleToken(style),
        ForegroundRole(role),
    )
}

/// An icon glyph.
pub fn glyph(code: &'static str, size: f32, role: ColorRole) -> impl Bundle {
    (
        icon_text(code, size, TextColor::default().0),
        ForegroundRole(role),
        A11yHidden,
        Pickable::IGNORE,
    )
}

/// A flex column that may shrink (`min` sizes 0).
pub fn column(gap: f32) -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(gap),
        min_width: Val::Px(0.0),
        min_height: Val::Px(0.0),
        ..default()
    }
}

/// A flex row that may shrink.
pub fn row(gap: f32) -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(gap),
        align_items: AlignItems::Center,
        min_width: Val::Px(0.0),
        min_height: Val::Px(0.0),
        ..default()
    }
}

/// How prominent an action button is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emphasis {
    /// The one primary action of a screen.
    Primary,
    /// A normal action.
    Tonal,
    /// A quiet action.
    Quiet,
}

/// Spawns a button with a keyed label, an action and a stable id; returns it.
pub fn action_button(
    commands: &mut Commands,
    key: &'static str,
    emphasis: Emphasis,
    action: PlannerAction,
    id: String,
) -> Entity {
    let builder = match emphasis {
        Emphasis::Primary => ButtonBuilder::filled(key),
        Emphasis::Tonal => ButtonBuilder::tonal(key),
        Emphasis::Quiet => ButtonBuilder::text(key),
    }
    .with_key(key);
    let button = builder.spawn(commands);
    commands.entity(button).insert((Act(action), UiId::new(id)));
    button
}
