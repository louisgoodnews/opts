//! The equipment dialog (*Configure…*): one switch per option the vehicle's repaints offer.

use bevy::prelude::*;
use opts_core::setvars::SetvarKind;
use opts_gui::components::combobox::ComboboxBuilder;
use opts_gui::components::modal::ModalEvent;
use opts_gui::components::text_field::TextFieldBuilder;
use opts_gui::interaction::events::{ComboboxSelected, SwitchToggled, TextChanged};
use opts_gui::l10n::Localization;
use opts_gui::prelude::*;

use super::livery::LiveryState;
use super::setvars_discovery::{setvars_from, SetvarItem};
use super::types::{GameCatalogue, NewGameSetup};
use super::ui::{action_button, column, keyed, plain, row, Emphasis, PlannerAction};

mod apply;
mod controls;

pub use apply::{apply_choices, apply_numbers, apply_switches, close_on_cancel};
use controls::option_line;

/// Marks the backdrop of the dialog.
#[derive(Component, Debug, Clone, Copy)]
pub struct OptionsModal;

/// The switch of one option.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct OptionSwitch {
    /// Option name (the script variable).
    pub name: String,
    /// The value the livery gives it.
    pub default: i32,
}

/// The choice box of an option with several values.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct OptionChoice {
    /// Option name (the script variable).
    pub name: String,
    /// The value the livery gives it.
    pub default: i32,
    /// The values, in the order of the box.
    pub values: Vec<i32>,
}

/// The number field of an option that takes a free number.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct OptionNumber {
    /// Option name (the script variable).
    pub name: String,
    /// The value the livery gives it.
    pub default: i32,
    /// Smallest accepted value.
    pub min: i32,
    /// Largest accepted value.
    pub max: i32,
}

/// The footer with *Reset* and *Done*.
fn options_footer(commands: &mut Commands) -> Entity {
    let footer = commands
        .spawn(Node {
            justify_content: JustifyContent::FlexEnd,
            ..row(12.0)
        })
        .id();
    let reset = action_button(
        commands,
        "wiz-options-reset",
        Emphasis::Quiet,
        PlannerAction::ResetOptions,
        "wizard.what.options.reset".into(),
    );
    let done = action_button(
        commands,
        "wiz-options-done",
        Emphasis::Primary,
        PlannerAction::CloseOptions,
        "wizard.what.options.done".into(),
    );
    commands.entity(done).insert(DefaultFocus);
    commands.entity(footer).add_children(&[reset, done]);
    footer
}

/// Opens the dialog for the chosen vehicle (nothing without discovered options).
pub fn open_options(
    commands: &mut Commands,
    loc: &Localization,
    catalogue: &GameCatalogue,
    setup: &NewGameSetup,
    liveries: &LiveryState,
) {
    let Some(reps) = &liveries.repaints else {
        return;
    };
    let items = setvars_from(reps, &setup.custom_setvars, &setup.selected_livery);
    if items.is_empty() {
        return;
    }
    let (root, content) = ModalBuilder::new()
        .with_size_percent(50.0, 70.0)
        .with_width_range(420.0, 640.0)
        .spawn(commands);
    commands
        .entity(root)
        .insert((OptionsModal, UiId::new("wizard.what.options")));
    let title = commands
        .spawn(keyed(
            "wiz-options-title",
            TypeScale::TitleLarge,
            ColorRole::OnSurface,
        ))
        .id();
    let name = super::texts::what_text(loc, catalogue, setup);
    let sub = commands
        .spawn(plain(
            name,
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let list = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                overflow: Overflow::scroll_y(),
                ..column(4.0)
            },
            Name::new("OptionsList"),
        ))
        .id();
    opts_gui::patterns::scroll::wheel_scroll(commands, list);
    for item in &items {
        let line = option_line(commands, item);
        commands.entity(list).add_child(line);
    }
    let footer = options_footer(commands);
    commands
        .entity(content)
        .add_children(&[title, sub, list, footer]);
    let _ = loc;
}

/// Closes the dialog.
/// *Reset*: clears the choices and rebuilds the open dialog (its controls kept showing the
/// cleared choices until it was closed).
pub(super) fn reset_options(env: &mut super::systems::Env, commands: &mut Commands) {
    env.setup.custom_setvars.clear();
    close_options(&env.modals, commands);
    open_options(
        commands,
        &env.loc,
        &env.catalogue,
        &env.setup,
        &env.liveries,
    );
}

pub fn close_options(modals: &Query<Entity, With<OptionsModal>>, commands: &mut Commands) {
    for e in modals {
        commands.entity(e).despawn();
    }
}
