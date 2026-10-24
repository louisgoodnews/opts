//! The train composer of the wizard (*Compose a train*): a dialog over the What step in which cars
//! of the installation are added behind the chosen rail vehicle, turned, moved and removed. The
//! train is kept in [`NewGameSetup::formation`] and built when the drive starts
//! (`apply_to_start`).

use bevy::prelude::*;
use opts_gui::components::button::ButtonBuilder;
use opts_gui::l10n::Localization;
use opts_gui::prelude::*;

use super::types::{GameCatalogue, NewGameSetup, VehicleItem};
use super::ui::{action_button, column, keyed, plain, row, Act, Emphasis, PlannerAction};

/// Marks the backdrop of the composer.
#[derive(Component, Debug, Clone, Copy)]
pub struct FormationModal;

/// The chosen vehicle, as the first car, when no train has been started yet.
pub fn ensure_head(setup: &mut NewGameSetup, catalogue: &GameCatalogue) {
    if !setup.formation.is_empty() {
        return;
    }
    if let Some(v) = catalogue.find_vehicle(&setup.selected_vehicle) {
        if !v.bus_file.is_empty() {
            setup.formation.push((v.bus_file.clone(), false));
        }
    }
}

/// Closes the composer.
pub fn close(modals: &Query<Entity, With<FormationModal>>, commands: &mut Commands) {
    for e in modals {
        commands.entity(e).despawn();
    }
}

/// Builds the composer again from the setup (after every change).
pub fn reopen(
    commands: &mut Commands,
    modals: &Query<Entity, With<FormationModal>>,
    loc: &Localization,
    catalogue: &GameCatalogue,
    setup: &NewGameSetup,
) {
    close(modals, commands);
    open(commands, loc, catalogue, setup);
}

fn name_of(catalogue: &GameCatalogue, file: &str) -> String {
    catalogue
        .vehicles
        .iter()
        .find(|v| v.bus_file == file)
        .map_or_else(|| file.to_owned(), |v| v.full_name.clone())
}

fn small_button(
    commands: &mut Commands,
    key: &'static str,
    action: PlannerAction,
    id: String,
) -> Entity {
    action_button(commands, key, Emphasis::Quiet, action, id)
}

/// Opens the composer.
pub fn open(
    commands: &mut Commands,
    loc: &Localization,
    catalogue: &GameCatalogue,
    setup: &NewGameSetup,
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(60.0, 80.0)
        .with_width_range(520.0, 820.0)
        .spawn(commands);
    commands
        .entity(root)
        .insert((FormationModal, UiId::new("wizard.what.formation.modal")));
    let title = commands
        .spawn(keyed(
            "wiz-formation-title",
            TypeScale::TitleLarge,
            ColorRole::OnSurface,
        ))
        .id();
    let hint = commands
        .spawn(keyed(
            "wiz-formation-hint",
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    // The train so far.
    let train = commands
        .spawn((
            Node {
                max_height: Val::Percent(40.0),
                overflow: Overflow::scroll_y(),
                ..column(4.0)
            },
            Name::new("FormationCars"),
        ))
        .id();
    opts_gui::patterns::scroll::wheel_scroll(commands, train);
    for (i, (file, turned)) in setup.formation.iter().enumerate() {
        let line = commands
            .spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                width: Val::Percent(100.0),
                min_height: Val::Px(44.0),
                flex_shrink: 0.0,
                ..row(8.0)
            })
            .id();
        let mut label = format!("{}  {}", i + 1, name_of(catalogue, file));
        if i == 0 {
            label.push_str(&format!("  ({})", loc.t("wiz-formation-head")));
        }
        if *turned {
            label.push_str(&format!("  ({})", loc.t("wiz-formation-turned")));
        }
        let text = commands
            .spawn((
                plain(label, TypeScale::BodyMedium, ColorRole::OnSurface),
                Node {
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    ..default()
                },
            ))
            .id();
        let mut kids = vec![text];
        if i > 1 {
            kids.push(small_button(
                commands,
                "wiz-formation-up",
                PlannerAction::FormationUp(i),
                format!("wizard.what.formation.up.{i}"),
            ));
        }
        kids.push(small_button(
            commands,
            "wiz-formation-turn",
            PlannerAction::FormationTurn(i),
            format!("wizard.what.formation.turn.{i}"),
        ));
        if i > 0 {
            kids.push(small_button(
                commands,
                "wiz-formation-remove",
                PlannerAction::FormationRemove(i),
                format!("wizard.what.formation.remove.{i}"),
            ));
        }
        commands.entity(line).add_children(&kids);
        commands.entity(train).add_child(line);
    }
    // The cars that can be added.
    let add_title = commands
        .spawn(keyed(
            "wiz-formation-add",
            TypeScale::TitleMedium,
            ColorRole::OnSurface,
        ))
        .id();
    let list = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                overflow: Overflow::scroll_y(),
                ..column(2.0)
            },
            Name::new("FormationAddList"),
        ))
        .id();
    opts_gui::patterns::scroll::wheel_scroll(commands, list);
    let mut cars: Vec<&VehicleItem> = catalogue
        .vehicles
        .iter()
        .filter(|v| v.class == "rail" && !v.bus_file.is_empty())
        .collect();
    cars.sort_by(|a, b| a.full_name.cmp(&b.full_name));
    for v in cars {
        let button = ButtonBuilder::text(v.full_name.clone()).spawn(commands);
        commands.entity(button).insert((
            Act(PlannerAction::FormationAdd(v.bus_file.clone())),
            UiId::new(format!(
                "wizard.what.formation.add.{}",
                opts_gui::ui_id::slug(&v.id)
            )),
            Node {
                flex_shrink: 0.0,
                justify_content: JustifyContent::FlexStart,
                ..default()
            },
        ));
        commands.entity(list).add_child(button);
    }
    let footer = commands
        .spawn(Node {
            justify_content: JustifyContent::FlexEnd,
            ..row(12.0)
        })
        .id();
    let clear = action_button(
        commands,
        "wiz-formation-clear",
        Emphasis::Quiet,
        PlannerAction::FormationClear,
        "wizard.what.formation.clear".into(),
    );
    let done = action_button(
        commands,
        "wiz-formation-done",
        Emphasis::Primary,
        PlannerAction::CloseFormation,
        "wizard.what.formation.done".into(),
    );
    commands.entity(done).insert(DefaultFocus);
    commands.entity(footer).add_children(&[clear, done]);
    commands
        .entity(content)
        .add_children(&[title, hint, train, add_title, list, footer]);
}

/// Sets the cars behind the head for the drive that starts, and returns the train to record with
/// the vehicle (`None` for a vehicle alone or a train that does not resolve).
pub fn apply_to_start(
    setup: &NewGameSetup,
    job: &mut crate::in_game::omsi_world::WorldLoadJob,
) -> Option<opts_core::formats::trains::TrainFormation> {
    if setup.formation.len() < 2 {
        return None;
    }
    let mut train = opts_core::formats::trains::TrainFormation::custom("Formation");
    for (file, turned) in &setup.formation {
        let at = train.add_file(file);
        if *turned {
            train.turn_car(at);
        }
    }
    match crate::formation::resolve(&train, job) {
        Ok(resolved) => {
            job.set_formation_cars(resolved.cars);
            train.name = train.suggested_name();
            Some(train)
        }
        Err(issues) => {
            let text: Vec<String> = issues.iter().map(ToString::to_string).collect();
            tracing::warn!("the composed train cannot be started: {}", text.join("; "));
            None
        }
    }
}
