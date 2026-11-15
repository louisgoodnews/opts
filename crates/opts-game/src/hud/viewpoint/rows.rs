//! The value rows of the viewpoint editor: marker, name, step buttons, track and value.

use super::*;

/// One value row: marker, name, *minus*, the track with its fill, *plus* and the value.
pub(super) fn value_row(
    p: &mut ChildSpawnerCommands,
    (row, name_key): (ViewpointRow, &'static str),
    palette: &InstrumentPalette,
) {
    p.spawn(Node {
        align_items: AlignItems::Center,
        column_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|r| {
        r.spawn((
            texts("›", TypeScale::BodyMedium, InstrumentInk::Needle, palette),
            ViewpointMarker(row),
            Node {
                width: Val::Px(10.0),
                ..default()
            },
        ));
        r.spawn((
            texts(name_key, TypeScale::Label, InstrumentInk::TextDim, palette),
            L10nKey(name_key),
            ViewpointName(row),
            Node {
                width: Val::Px(86.0),
                ..default()
            },
        ));
        small_button(
            r,
            ViewpointButton::Step(row, -1),
            keys::VIEWPOINT_DECREASE,
            format!("hud.viewpoint.{}.minus", row.id()),
            palette,
        );
        track(r, row, palette);
        small_button(
            r,
            ViewpointButton::Step(row, 1),
            keys::VIEWPOINT_INCREASE,
            format!("hud.viewpoint.{}.plus", row.id()),
            palette,
        );
        r.spawn((
            texts("", TypeScale::BodyMedium, InstrumentInk::Text, palette),
            ViewpointValue(row),
            UiId::new(format!("hud.viewpoint.{}.value", row.id())),
            Node {
                min_width: Val::Px(64.0),
                ..default()
            },
        ));
    });
}

/// The track of a value row: a clickable bar with the fill that shows the value.
pub(super) fn track(r: &mut ChildSpawnerCommands, row: ViewpointRow, palette: &InstrumentPalette) {
    r.spawn((
        Node {
            width: Val::Px(92.0),
            height: Val::Px(12.0),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(palette.track),
        BorderColor::all(palette.panel_border),
        Button,
        PointerOnly,
        RelativeCursorPosition::default(),
        ViewpointTrack(row),
        ViewpointPaint(false),
        RadiusToken(RadiusLevel::Full),
        UiId::new(format!("hud.viewpoint.{}.track", row.id())),
    ))
    .with_children(|t| {
        t.spawn((
            Node {
                width: Val::Percent(50.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(palette.needle),
            ViewpointFill(row),
            ViewpointPaint(true),
            RadiusToken(RadiusLevel::Full),
        ));
    });
}
