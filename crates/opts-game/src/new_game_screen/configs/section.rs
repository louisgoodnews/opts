//! Building the Configurations section of the What step: title, name row and the list.

use super::*;

/// Builds the section: title, name field with *Save*, then the list.
pub fn spawn_configs_section(
    commands: &mut Commands,
    loc: &Localization,
    library: &ConfigLibrary,
    catalogue: &GameCatalogue,
    setup: &NewGameSetup,
    vehicle: &VehicleItem,
    repaints: Option<&VehicleRepaints>,
) -> Entity {
    let section = commands.spawn(column(8.0)).id();
    let mut kids = vec![commands
        .spawn(keyed(
            "wiz-configs-title",
            TypeScale::TitleMedium,
            ColorRole::OnSurface,
        ))
        .id()];
    if let Some(problem) = &library.problem {
        kids.push(
            commands
                .spawn(keyed_args(
                    "wiz-configs-file-problem",
                    L10nArgs::one("reason", problem.to_string()),
                    TypeScale::BodyMedium,
                    ColorRole::Warning,
                ))
                .id(),
        );
    }
    let name_row = name_row(commands, loc, library, setup, vehicle);
    kids.push(name_row);

    if library.store.configurations.is_empty() {
        kids.push(
            commands
                .spawn(keyed(
                    "wiz-configs-empty",
                    TypeScale::BodyMedium,
                    ColorRole::OnSurfaceVariant,
                ))
                .id(),
        );
    }
    // Favourites first, then the most recently used.
    let mut order: Vec<&BusConfiguration> = library.store.configurations.iter().collect();
    order.sort_by_key(|c| (!c.favourite, std::cmp::Reverse(c.last_used)));
    for (i, c) in order.into_iter().enumerate() {
        kids.push(config_row(
            commands,
            library,
            catalogue,
            c,
            (vehicle, repaints),
            i,
        ));
    }
    commands.entity(section).add_children(&kids);
    section
}
