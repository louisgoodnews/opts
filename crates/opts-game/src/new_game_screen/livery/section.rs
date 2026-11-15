//! The livery section of the vehicle detail: its title, loading line and boxes.

use super::*;

/// The livery section of the detail: title, loading line and the boxes the poll system fills.
pub fn spawn_livery_section(
    commands: &mut Commands,
    state: &mut LiveryState,
    setup: &NewGameSetup,
    vehicle: &VehicleItem,
) -> Entity {
    request(state, vehicle);
    let section = commands.spawn(column(8.0)).id();
    let title = commands
        .spawn(keyed(
            "wiz-what-livery",
            TypeScale::TitleMedium,
            ColorRole::OnSurface,
        ))
        .id();
    let liveries = commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Stretch,
                ..row(8.0)
            },
            LiveryBox(vehicle.id.clone()),
        ))
        .id();
    let options = commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                ..row(12.0)
            },
            OptionsBox(vehicle.id.clone()),
        ))
        .id();
    match (&state.repaints, state.vehicle == vehicle.id) {
        (Some(reps), true) => {
            let reps = reps.clone();
            fill_liveries(commands, liveries, vehicle, &reps);
            fill_options(commands, options, &reps, setup);
        }
        _ => {
            let loading = commands
                .spawn(keyed(
                    "wiz-what-livery-loading",
                    TypeScale::BodyMedium,
                    ColorRole::OnSurfaceVariant,
                ))
                .id();
            commands.entity(liveries).add_child(loading);
        }
    }
    commands
        .entity(section)
        .add_children(&[title, liveries, options]);
    section
}
