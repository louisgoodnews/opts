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

/// The control that fits `item`: a switch, a choice box or a number field.
fn option_control(commands: &mut Commands, item: &SetvarItem) -> Entity {
    let id = format!("wizard.what.option.{}", opts_gui::ui_id::slug(&item.name));
    match item.kind {
        SetvarKind::Choice if !item.values.is_empty() => {
            let at = item
                .values
                .iter()
                .position(|v| *v == item.value)
                .unwrap_or(0);
            let combo = ComboboxBuilder::new(item.label.clone())
                .with_options(item.values.iter().map(i32::to_string).collect())
                .with_selected(at)
                .spawn(commands);
            commands.entity(combo).insert((
                OptionChoice {
                    name: item.name.clone(),
                    default: item.default_value,
                    values: item.values.clone(),
                },
                UiId::new(id),
            ));
            combo
        }
        SetvarKind::Number | SetvarKind::Choice => {
            let field = TextFieldBuilder::outlined(item.label.clone())
                .with_text(item.value.to_string())
                .with_max_characters(8)
                .spawn(commands);
            commands.entity(field).insert((
                OptionNumber {
                    name: item.name.clone(),
                    default: item.default_value,
                    min: item.min_val,
                    max: item.max_val,
                },
                UiId::new(id),
            ));
            field
        }
        SetvarKind::Switch | SetvarKind::Fraction => {
            let switch = SwitchBuilder::new(item.label.clone())
                .with_value(item.value != 0)
                .spawn(commands);
            commands.entity(switch).insert((
                OptionSwitch {
                    name: item.name.clone(),
                    default: item.default_value,
                },
                UiId::new(id),
            ));
            switch
        }
    }
}


/// One option: a line with its control and, for a choice box or number field, its name.
fn option_line(commands: &mut Commands, item: &SetvarItem) -> Entity {
    // A row never shrinks below its control (the list scrolls instead), and a choice box or a
    // number field gets its name as a label beside it: the box shows only the value.
    let line = commands
        .spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            width: Val::Percent(100.0),
            min_height: Val::Px(56.0),
            flex_shrink: 0.0,
            ..row(12.0)
        })
        .id();
    let control = option_control(commands, item);
    if !matches!(item.kind, SetvarKind::Switch | SetvarKind::Fraction) {
        let label = commands
            .spawn((
                plain(
                    item.label.clone(),
                    TypeScale::BodyMedium,
                    ColorRole::OnSurface,
                ),
                Node {
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    ..default()
                },
            ))
            .id();
        commands.entity(line).add_child(label);
    }
    commands.entity(line).add_child(control);
    line
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
        set_override(&mut setup, &s.name, s.default, i32::from(t.is_on));
    }
}

/// Writes a chosen value of a choice box into the choices.
pub fn apply_choices(
    mut chosen: MessageReader<ComboboxSelected>,
    boxes: Query<&OptionChoice>,
    mut setup: ResMut<NewGameSetup>,
) {
    for c in chosen.read() {
        let Ok(b) = boxes.get(c.entity) else {
            continue;
        };
        let Some(value) = b.values.get(c.selected_index).copied() else {
            continue;
        };
        set_override(&mut setup, &b.name, b.default, value);
    }
}

/// Writes an edited number into the choices (text that is not a number in range is ignored).
pub fn apply_numbers(
    mut edited: MessageReader<TextChanged>,
    fields: Query<&OptionNumber>,
    mut setup: ResMut<NewGameSetup>,
) {
    for t in edited.read() {
        let Ok(f) = fields.get(t.entity) else {
            continue;
        };
        let Ok(value) = t.text.trim().parse::<i32>() else {
            continue;
        };
        if (f.min..=f.max).contains(&value) {
            set_override(&mut setup, &f.name, f.default, value);
        }
    }
}

/// A value equal to the livery's own is no override.
fn set_override(setup: &mut NewGameSetup, name: &str, default: i32, value: i32) {
    if value == default {
        setup.custom_setvars.remove(name);
    } else {
        setup.custom_setvars.insert(name.to_string(), value);
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
