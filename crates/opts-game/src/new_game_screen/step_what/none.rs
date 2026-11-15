//! The detail of *Drive without a vehicle*, and the formation entry of a rail vehicle.

use super::*;

/// The detail of *Drive without a vehicle*: what the session will be like.
pub fn fill_none_detail(commands: &mut Commands, detail: Entity) {
    let title = commands
        .spawn(keyed(
            "wiz-what-none",
            TypeScale::TitleLarge,
            ColorRole::OnSurface,
        ))
        .id();
    let body = commands
        .spawn(keyed(
            "wiz-what-none-body",
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    commands.entity(detail).add_children(&[title, body]);
}

/// For a rail vehicle: the note and the button that opens the formation editor.
pub(super) fn formation_entry(commands: &mut Commands, vehicle: &VehicleItem) -> Vec<Entity> {
    let mut out = Vec::new();
    if vehicle.class == "rail" {
        out.push(
            commands
                .spawn(keyed(
                    "wiz-what-formation-note",
                    TypeScale::BodyMedium,
                    ColorRole::OnSurfaceVariant,
                ))
                .id(),
        );
        out.push(super::super::ui::action_button(
            commands,
            "wiz-what-formation",
            super::super::ui::Emphasis::Tonal,
            super::super::ui::PlannerAction::ComposeFormation,
            "wizard.what.formation".to_owned(),
        ));
    }
    out
}
