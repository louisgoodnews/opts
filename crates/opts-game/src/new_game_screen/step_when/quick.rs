//! The quick-pick chips of the *When* step.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::new_game_screen::choice::{ChoiceStep, spawn_choice};
use crate::new_game_screen::planner::{PlannerStep, QUICK_TIMES};
use crate::new_game_screen::ui::{PlannerAction, keyed, row};

pub(super) fn quick_chips(commands: &mut Commands) -> Entity {
    let line = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            ..row(8.0)
        })
        .id();
    for (i, q) in QUICK_TIMES.iter().enumerate() {
        let chip = spawn_choice(
            commands,
            PlannerAction::Quick(i),
            format!("wizard.when.quick.{i}"),
            Node {
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                ..default()
            },
        );
        let text = commands
            .spawn(keyed(q.key, TypeScale::BodyMedium, ColorRole::OnSurface))
            .id();
        commands
            .entity(chip)
            .insert((
                ChoiceStep(PlannerStep::When),
                RadiusToken(RadiusLevel::Full),
            ))
            .insert_children(1, &[text]);
        commands.entity(line).add_child(chip);
    }
    line
}
