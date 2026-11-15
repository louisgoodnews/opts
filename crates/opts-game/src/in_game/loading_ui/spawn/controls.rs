//! The controls card of the loading screen: the keys a driver needs while waiting.

use super::super::*;

/// The keys a driver needs while waiting (the bindings that exist today).
const CONTROLS: [(&str, &str); 8] = [
    ("W / ↑", "load-controls-accelerate"),
    ("S / ↓", "load-controls-brake"),
    ("← →", "load-controls-steer"),
    (".", "load-controls-parking"),
    ("D  N  R", "load-controls-gears"),
    ("1 – 5", "load-controls-cameras"),
    ("P", "load-controls-pause"),
    ("Esc", "load-controls-menu"),
];

pub(super) fn controls_card(commands: &mut Commands) -> Entity {
    let card = commands
        .spawn((
            Node {
                padding: UiRect::all(Val::Px(12.0)),
                ..column(8.0)
            },
            BackgroundRole(ColorRole::SurfaceContainerLow),
            RadiusToken(RadiusLevel::Medium),
            Name::new("ControlsCard"),
        ))
        .id();
    let title = commands
        .spawn(keyed(
            "load-controls-title",
            TypeScale::Label,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let grid = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(24.0),
            row_gap: Val::Px(6.0),
            ..row(24.0)
        })
        .id();
    for (keys, action) in CONTROLS {
        let item = commands.spawn(row(8.0)).id();
        let chip = commands
            .spawn((
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                    ..default()
                },
                BackgroundRole(ColorRole::SurfaceContainerHighest),
                RadiusToken(RadiusLevel::Small),
            ))
            .id();
        let k = commands
            .spawn(plain(keys, TypeScale::Label, ColorRole::OnSurface))
            .id();
        commands.entity(chip).add_child(k);
        let a = commands
            .spawn(keyed(
                action,
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            ))
            .id();
        commands.entity(item).add_children(&[chip, a]);
        commands.entity(grid).add_child(item);
    }
    commands.entity(card).add_children(&[title, grid]);
    card
}
