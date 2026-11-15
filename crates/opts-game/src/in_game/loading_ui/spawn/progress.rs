//! The progress block of the loading screen: stage, percentage and the bar.

use super::super::*;

pub(super) fn progress_block(commands: &mut Commands) -> Entity {
    let block = commands.spawn(column(8.0)).id();
    let line = commands
        .spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            ..row(12.0)
        })
        .id();
    let stage = commands
        .spawn((
            keyed(
                "load-stage-resolve",
                TypeScale::BodyLarge,
                ColorRole::OnSurface,
            ),
            StageText,
        ))
        .id();
    let percent = commands
        .spawn((
            plain("0%", TypeScale::BodyLarge, ColorRole::Primary),
            PercentText,
        ))
        .id();
    commands.entity(line).add_children(&[stage, percent]);
    let track = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(12.0),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundRole(ColorRole::SurfaceContainerHighest),
            RadiusToken(RadiusLevel::Full),
        ))
        .id();
    let fill = commands
        .spawn((
            Node {
                width: Val::Percent(5.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundRole(ColorRole::Primary),
            RadiusToken(RadiusLevel::Full),
            ProgressBarFill,
        ))
        .id();
    commands.entity(track).add_child(fill);
    commands.entity(block).add_children(&[line, track]);
    block
}
