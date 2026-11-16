//! The fuel bar gauge (M-0123): a thin track with a fill; the update system sets the fill's width
//! and colours from the level.

use super::*;

/// Width of the gauge in layout px (the width of a readout column).
const BAR_WIDTH: f32 = 56.0;
/// Height of the gauge in layout px.
const BAR_HEIGHT: f32 = 6.0;

/// Builds the fuel bar: the track (the widget's root node) and its fill.
pub(in crate::hud) fn fuel_bar(
    parent: &mut ChildSpawnerCommands,
    w: &'static HudWidget,
    palette: &InstrumentPalette,
) {
    parent
        .spawn((
            Node {
                width: Val::Px(BAR_WIDTH),
                height: Val::Px(BAR_HEIGHT),
                ..default()
            },
            BackgroundColor(palette.panel_border),
            HudBarTrack,
            HudWidgetNode {
                id: w.id,
                slot: w.slot,
            },
            UiId::new(format!("hud.{}", w.id)),
        ))
        .with_children(|track| {
            track.spawn((
                Node {
                    width: Val::Percent(0.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(palette.ok),
                HudBarFill,
            ));
        });
}
