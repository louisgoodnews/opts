//! The start point selector and the description box of the chosen map in step Where.

use super::*;

pub(super) fn start_point(commands: &mut Commands, ctx: &BuildCtx, map: &MapItem) -> Entity {
    let wrap = commands.spawn(column(6.0)).id();
    if map.entry_points.is_empty() {
        let none = commands
            .spawn(keyed(
                "wiz-where-no-starts",
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ))
            .id();
        commands.entity(wrap).add_child(none);
        return wrap;
    }
    let label = commands
        .spawn(keyed(
            "wiz-where-start",
            TypeScale::Label,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let index = map
        .entry_points
        .iter()
        .position(|e| *e == ctx.setup.selected_entry_point)
        .unwrap_or(0);
    let combo = ComboboxBuilder::new(ctx.loc.t("wiz-where-start"))
        .with_options(map.entry_points.clone())
        .with_selected(index)
        .spawn(commands);
    commands
        .entity(combo)
        .insert((StartPointBox, UiId::new("wizard.where.start-point")));
    commands.entity(wrap).add_children(&[label, combo]);
    wrap
}

pub(super) fn description(commands: &mut Commands, map: &MapItem) -> Entity {
    let wrap = commands.spawn(column(4.0)).id();
    let label = commands
        .spawn(keyed(
            "wiz-where-description",
            TypeScale::Label,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let text = description_box(commands, map.description.trim());
    commands.entity(wrap).add_children(&[label, text]);
    wrap
}

/// A text box of limited height that scrolls with the wheel (long descriptions).
pub fn description_box(commands: &mut Commands, text: &str) -> Entity {
    let boxed = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_height: Val::Px(150.0),
                padding: UiRect::all(Val::Px(8.0)),
                overflow: Overflow::scroll_y(),
                flex_shrink: 0.0,
                ..column(0.0)
            },
            BackgroundRole(ColorRole::SurfaceContainer),
            RadiusToken(RadiusLevel::Small),
            Name::new("DescriptionBox"),
        ))
        .id();
    opts_gui::patterns::scroll::wheel_scroll(commands, boxed);
    let body = commands
        .spawn((
            plain(
                text.to_string(),
                TypeScale::BodyMedium,
                ColorRole::OnSurface,
            ),
            Node {
                width: Val::Percent(100.0),
                ..default()
            },
        ))
        .id();
    commands.entity(boxed).add_child(body);
    boxed
}
