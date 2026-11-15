//! The tabs that choose the list and the search box above it.

use super::*;

/// The three tabs that choose which list the column shows.
pub(super) fn list_tabs(c: &mut ChildSpawnerCommands, ed: &FormationEditor) {
    c.spawn(Node {
        flex_direction: FlexDirection::Row,
        column_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|tabs| {
        let cars = button(
            tabs,
            "Rail cars",
            ed.list == ListKind::Cars,
            Act::List(ListKind::Cars),
        );
        id(&mut tabs.commands(), cars, "formation.list.cars");
        let presets = button(
            tabs,
            "Trains",
            ed.list == ListKind::Presets,
            Act::List(ListKind::Presets),
        );
        id(&mut tabs.commands(), presets, "formation.list.presets");
        let saved = button(
            tabs,
            "Saved",
            ed.list == ListKind::Saved,
            Act::List(ListKind::Saved),
        );
        id(&mut tabs.commands(), saved, "formation.list.saved");
    });
}

/// The search box above the list.
pub(super) fn search_field(c: &mut ChildSpawnerCommands, ed: &FormationEditor) {
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
        let text = if ed.search.is_empty() {
            "Search: type to filter…".to_string()
        } else {
            format!("Search: {}|", ed.search)
        };
        label(f, text, 13.0, ColorRole::OnSurface);
    });
}
