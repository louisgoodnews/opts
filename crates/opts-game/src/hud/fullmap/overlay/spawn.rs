//! Spawning the overlay tree and the texts of its caption.

use super::*;

/// Spawns the overlay.
pub(super) fn spawn_overlay(commands: &mut Commands, palette: &InstrumentPalette, ground: Color) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(ground),
            GlobalZIndex(1000),
            InGameEntity,
            FullMapRoot,
            UiId::new("hud.fullmap"),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                ImageNode::default(),
                FullMapSurface,
                UiId::new("hud.fullmap.surface"),
            ));
            caption_panel(root, palette);
            scale_panel(root, palette);
            hint_panel(root, palette);
        });
}

/// The caption (the map name, with the reading progress while the overview is still being read)
/// and the text of the scale bar.
pub(super) fn caption_texts(
    loc: &Localization,
    overview: &super::overview::OverviewRoads,
    map: &str,
    bar_m: f64,
) -> (String, String) {
    let bar_text = if bar_m >= 1000.0 {
        loc.t_args(
            keys::FULLMAP_SCALE_KM,
            &[("km".to_owned(), Arg::from((bar_m / 1000.0).round() as i32))],
        )
    } else {
        loc.t_args(
            keys::FULLMAP_SCALE_M,
            &[("metres".to_owned(), Arg::from(bar_m.round() as i32))],
        )
    };
    let name = if map.is_empty() {
        loc.t(keys::FULLMAP_TITLE)
    } else {
        map.to_owned()
    };
    // While the reader works on the other tiles, the title says how far it is.
    let title = if overview.started() && !overview.complete() {
        loc.t_args(
            keys::FULLMAP_READING,
            &[
                ("map".to_owned(), Arg::from(name.as_str())),
                ("percent".to_owned(), Arg::from(overview.percent() as i32)),
            ],
        )
    } else {
        name
    };
    (title, bar_text)
}
