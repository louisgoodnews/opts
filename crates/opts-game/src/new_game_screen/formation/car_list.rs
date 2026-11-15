//! The composer's car list to add from, and its footer with *Clear* and *Done*.

use super::*;

/// The title and the list of the cars that can be added.
pub(super) fn spawn_car_list(commands: &mut Commands, catalogue: &GameCatalogue) -> (Entity, Entity) {
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
    (add_title, list)
}

/// The footer with *Clear* and *Done*.
pub(super) fn spawn_footer(commands: &mut Commands) -> Entity {
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
    footer
}
