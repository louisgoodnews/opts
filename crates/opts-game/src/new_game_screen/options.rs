//! The equipment dialog (*Configure…*): one switch per option the vehicle's repaints offer.

use bevy::prelude::*;
use opts_gui::components::modal::ModalEvent;
use opts_gui::interaction::events::SwitchToggled;
use opts_gui::l10n::Localization;
use opts_gui::prelude::*;

use super::livery::LiveryState;
use super::setvars_discovery::setvars_from;
use super::types::{GameCatalogue, NewGameSetup};
use super::ui::{action_button, column, keyed, plain, row, Emphasis, PlannerAction};

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
    let items = setvars_from(reps, &setup.custom_setvars);
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
        let line = commands
            .spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                width: Val::Percent(100.0),
                ..row(12.0)
            })
            .id();
        let switch = SwitchBuilder::new(item.label.clone())
            .with_value(item.value != 0)
            .spawn(commands);
        commands.entity(switch).insert((
            OptionSwitch {
                name: item.name.clone(),
                default: item.default_value,
            },
            UiId::new(format!(
                "wizard.what.option.{}",
                opts_gui::ui_id::slug(&item.name)
            )),
        ));
        commands.entity(line).add_child(switch);
        commands.entity(list).add_child(line);
    }
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
    commands
        .entity(content)
        .add_children(&[title, sub, list, footer]);
    let _ = loc;
}

/// Writes a toggled switch into the choices (a value equal to the default is no override).
pub fn apply_switches(
    mut toggled: MessageReader<SwitchToggled>,
    switches: Query<&OptionSwitch>,
    mut setup: ResMut<NewGameSetup>,
) {
    for t in toggled.read() {
        let Ok(s) = switches.get(t.entity) else {
            continue;
        };
        let value = i32::from(t.is_on);
        if value == s.default {
            setup.custom_setvars.remove(&s.name);
        } else {
            setup.custom_setvars.insert(s.name.clone(), value);
        }
    }
}

/// Closes the dialog on Escape.
pub fn close_on_cancel(
    mut events: MessageReader<ModalEvent>,
    modals: Query<Entity, With<OptionsModal>>,
    mut commands: Commands,
) {
    for ModalEvent::CancelRequested(e) in events.read() {
        if modals.contains(*e) {
            commands.entity(*e).despawn();
        }
    }
}

/// Closes the dialog.
pub fn close_options(modals: &Query<Entity, With<OptionsModal>>, commands: &mut Commands) {
    for e in modals {
        commands.entity(e).despawn();
    }
}
