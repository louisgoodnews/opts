//! The overlay's panels: the caption, the scale bar and the keys, with their shared text bundle.

use super::*;

pub(super) fn texts_bundle(
    key: &'static str,
    palette: &InstrumentPalette,
    ink: InstrumentInk,
) -> impl Bundle {
    (texts(key, TypeScale::Label, ink, palette), L10nKey(key))
}

/// The map name and the zoom, top left.
pub(super) fn caption_panel(root: &mut ChildSpawnerCommands, palette: &InstrumentPalette) {
    root.spawn(panel(
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        RadiusLevel::Medium,
        palette,
    ))
    .with_children(|p| {
        p.spawn((
            texts("", TypeScale::BodyLarge, InstrumentInk::Text, palette),
            FullMapCaption,
            UiId::new("hud.fullmap.caption"),
        ));
    });
}

/// The scale bar, bottom left.
pub(super) fn scale_panel(root: &mut ChildSpawnerCommands, palette: &InstrumentPalette) {
    root.spawn(panel(
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            bottom: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        RadiusLevel::Medium,
        palette,
    ))
    .with_children(|p| {
        p.spawn((
            Node {
                width: Val::Px(100.0),
                height: Val::Px(3.0),
                ..default()
            },
            BackgroundColor(palette.text),
            FullMapScaleBar,
        ));
        p.spawn((
            texts("", TypeScale::Label, InstrumentInk::TextDim, palette),
            FullMapScaleText,
            UiId::new("hud.fullmap.scale"),
        ));
    });
}

/// The keys, bottom right.
pub(super) fn hint_panel(root: &mut ChildSpawnerCommands, palette: &InstrumentPalette) {
    root.spawn(panel(
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            bottom: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        RadiusLevel::Medium,
        palette,
    ))
    .with_children(|p| {
        p.spawn((
            texts_bundle(keys::FULLMAP_HINT, palette, InstrumentInk::TextDim),
            UiId::new("hud.fullmap.hint"),
        ));
    });
}
