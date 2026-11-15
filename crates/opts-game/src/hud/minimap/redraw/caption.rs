//! The caption below the mini map: its radius and what points up.

use super::super::*;

/// Writes the caption below the map: the radius and what points up.
pub fn update_caption(
    state: Res<MiniMapState>,
    loc: Res<Localization>,
    mut captions: Query<&mut Text, With<MiniMapCaption>>,
) {
    let mode = match state.config.orientation {
        Orientation::North => keys::MINIMAP_NORTH_UP,
        Orientation::Heading => keys::MINIMAP_HEADING_UP,
    };
    // Speed zoom: the radius being drawn (to 10 m), marked as automatic.
    let (key, metres) = match state.config.zoom_mode {
        ZoomMode::Fixed => (keys::MINIMAP_CAPTION, state.config.zoom_m as i32),
        ZoomMode::Speed => (
            keys::MINIMAP_CAPTION_AUTO,
            if state.radius_m > 0.0 {
                (state.radius_m / 10.0).round() as i32 * 10
            } else {
                state.config.zoom_m as i32
            },
        ),
    };
    let line = loc.t_args(
        key,
        &[
            ("metres".to_owned(), Arg::from(metres)),
            ("mode".to_owned(), Arg::from(loc.t(mode))),
        ],
    );
    for mut text in &mut captions {
        if text.0 != line {
            text.0 = line.clone();
        }
    }
}
