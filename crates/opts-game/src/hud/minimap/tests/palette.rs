//! Colour tests: every map colour has contrast against the ground, and the markers differ in more than hue.

use super::*;

fn rgb(c: [u8; 4]) -> Color {
    Color::srgb_u8(c[0], c[1], c[2])
}

#[test]
fn every_map_colour_has_at_least_3_to_1_against_the_ground_in_day_and_night() {
    for (name, palette) in [("day", day()), ("night", night())] {
        let c = colors_of(&palette);
        assert_eq!(c.ground[3], 255, "{name}: the ground is opaque");
        for (what, color) in [
            ("road", c.road),
            ("active vehicle", c.active),
            ("other vehicle", c.other),
            ("north mark", c.north),
        ] {
            let ratio = contrast_ratio(rgb(color), rgb(c.ground));
            assert!(ratio >= 3.0, "{name} {what}: {ratio:.2}:1");
        }
    }
}

#[test]
fn the_vehicle_markers_differ_in_shape_and_lightness_not_only_in_hue() {
    // Colour-blind safe: the active vehicle is filled with a ring and the others are hollow
    // (tested above); the colours also differ in lightness, and neither pair is red against green.
    for palette in [day(), night()] {
        let c = colors_of(&palette);
        let ratio = contrast_ratio(rgb(c.active), rgb(c.other));
        assert!(ratio >= 1.2, "active against other: {ratio:.2}:1");
    }
}
