//! The train so far in the composer: one line per car with its move, turn and remove buttons.

use super::*;

pub(super) fn name_of(catalogue: &GameCatalogue, file: &str) -> String {
    catalogue
        .vehicles
        .iter()
        .find(|v| v.bus_file == file)
        .map_or_else(|| file.to_owned(), |v| v.full_name.clone())
}

pub(super) fn small_button(
    commands: &mut Commands,
    key: &'static str,
    action: PlannerAction,
    id: String,
) -> Entity {
    action_button(commands, key, Emphasis::Quiet, action, id)
}

/// The train so far: one line per car with its *Up*, *Turn* and *Remove* buttons.
pub(super) fn spawn_train(
    commands: &mut Commands,
    loc: &Localization,
    catalogue: &GameCatalogue,
    setup: &NewGameSetup,
) -> Entity {
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
    train
}
