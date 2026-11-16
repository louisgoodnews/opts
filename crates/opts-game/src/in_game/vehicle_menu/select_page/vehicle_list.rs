//! Columns 1 and 2 of the Select Vehicle modal: the vehicle list and the chosen vehicle's details.

use super::*;

/// Column 1: the vehicle list with its search field.
pub(super) fn vehicle_column(
    row: &mut ChildSpawnerCommands,
    menu: &VehicleMenu,
    cat: &GameCatalogue,
) {
    let c1 = column(row, 30.0);
    row.commands().entity(c1).with_children(|c| {
        let shown: Vec<usize> = cat
            .vehicles
            .iter()
            .enumerate()
            .filter(|(_, v)| matches_search(v, &menu.search))
            .map(|(i, _)| i)
            .collect();
        label(
            c,
            if menu.search.is_empty() {
                format!("Vehicles ({})", cat.vehicles.len())
            } else {
                format!("Vehicles ({} of {})", shown.len(), cat.vehicles.len())
            },
            15.0,
            ColorRole::Primary,
        );
        // The search field: type to filter, Backspace deletes (Escape steps back).
        c.spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderRole(ColorRole::Outline),
            BackgroundRole(ColorRole::SurfaceContainerLowest),
            RadiusToken(RadiusLevel::Small),
        ))
        .with_children(|f| {
            let text = if menu.search.is_empty() {
                "Search: type to filter…".to_string()
            } else {
                format!("Search: {}|", menu.search)
            };
            label(f, text, 13.0, ColorRole::OnSurface);
        });
        scroll_list(c, 0, menu.scroll[0], Val::Auto, |list| {
            if shown.is_empty() {
                label(
                    list,
                    "No vehicle matches.",
                    12.0,
                    ColorRole::OnSurfaceVariant,
                );
            }
            for i in shown {
                let v = &cat.vehicles[i];
                let selected = menu.vehicle == Some(i);
                button(list, &v.full_name, selected, Action::PickVehicle(i));
            }
        });
    });
}

pub(super) fn detail_column(c: &mut ChildSpawnerCommands, menu: &VehicleMenu, v: &VehicleItem) {
    label(c, v.full_name.clone(), 20.0, ColorRole::Primary);
    label(
        c,
        format!("{} · {}", v.manufacturer, v.model),
        12.0,
        ColorRole::OnSurfaceVariant,
    );
    // Only facts the content provides are shown (no "N/A" rows).
    if let Some(t) = v.mass_tonnes {
        label(
            c,
            format!("Empty weight: {t:.1} t"),
            12.0,
            ColorRole::OnSurface,
        );
    }
    scroll_list(c, 3, menu.scroll[3], Val::Px(110.0), |list| {
        label(list, v.description.clone(), 12.0, ColorRole::OnSurface);
    });
    label(c, "Repaint", 15.0, ColorRole::Primary);
    let names: Vec<String> = menu
        .repaints
        .as_ref()
        .map(|(_, r)| r.names.clone())
        .unwrap_or_default();
    scroll_list(c, 1, menu.scroll[1], Val::Auto, |list| {
        if names.is_empty() {
            label(
                list,
                "This vehicle has no repaints.",
                12.0,
                ColorRole::OnSurfaceVariant,
            );
        }
        for (i, name) in names.iter().enumerate() {
            let selected = if menu.repaint.is_empty() {
                i == 0
            } else {
                menu.repaint == *name
            };
            // The first (default) repaint is chosen with an empty name.
            let value = if i == 0 { String::new() } else { name.clone() };
            button(list, name, selected, Action::PickRepaint(value));
        }
    });
    button(c, "Configure vehicle…", false, Action::OpenConfig);
}
