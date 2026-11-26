//! Copies the drawn map into the image, records the redraw and picks the colours.

use super::super::*;

/// Copies the canvas into the image the HUD shows.
pub(super) fn upload(images: &mut Assets<Image>, handle: &Handle<Image>, canvas: &raster::Canvas) {
    if let Some(mut image) = images.get_mut(handle) {
        match image.data.as_mut() {
            Some(data) if data.len() == canvas.data.len() => data.copy_from_slice(&canvas.data),
            _ => image.data = Some(canvas.data.clone()),
        }
    }
}

/// What the state remembers of this redraw.
pub(super) fn record(
    state: &mut MiniMapState,
    started: std::time::Instant,
    (lines, markers): (usize, usize),
    (centre, up_heading): (DVec2, f32),
) {
    let us = started.elapsed().as_micros() as u64;
    state.frames += 1;
    state.last_us = us;
    state.worst_us = state.worst_us.max(us);
    state.lines = lines;
    state.markers = markers;
    state.centre = Some(centre);
    state.up_heading_deg = up_heading.to_degrees().rem_euclid(360.0);
}

/// The map colours of the current theme and variant.
pub(super) fn look_colors(look: &super::super::params::MapLook) -> MapColors {
    colors_of(
        look.theme
            .as_ref()
            .map_or(&opts_gui::theme::InstrumentTokens::default().day, |t| {
                t.instrument(*look.variant)
            }),
    )
}

/// The stroke colour of the timetable course: the needle token of the current theme and variant,
/// which is neither the route's `ok` green nor the other vehicles' `warning`.
pub(super) fn look_course_color(look: &super::super::params::MapLook) -> [u8; 4] {
    let needle = look
        .theme
        .as_ref()
        .map_or(&opts_gui::theme::InstrumentTokens::default().day, |t| {
            t.instrument(*look.variant)
        })
        .needle;
    needle.to_srgba().to_u8_array()
}
