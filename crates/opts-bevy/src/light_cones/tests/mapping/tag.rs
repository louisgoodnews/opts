//! The packed cone tag: colour, level, inner edge and ground height.

use super::*;

#[test]
fn the_tag_packs_colour_level_inner_edge_and_ground_height() {
    let tag = pack_tag([1.0, 0.5, 0.0], 0.5, 0.2, Some(7.0));
    assert_eq!(tag & 31, 31);
    assert_eq!((tag >> 5) & 31, 16);
    assert_eq!((tag >> 10) & 31, 0);
    assert_eq!(tag_level(tag), 32);
    assert_eq!((tag >> 21) & 15, 3);
    assert_eq!(tag_ground_h(tag), Some(7.0));
    assert_eq!(tag_ground_h(pack_tag([0.0; 3], 1.0, 0.0, None)), None);
    assert_eq!(
        tag_level(pack_tag([0.0; 3], 2.0, 0.0, None)),
        63,
        "level is clamped"
    );
    assert_eq!(tag_level(pack_tag([0.0; 3], -1.0, 0.0, None)), 0);
    assert_eq!(
        tag_ground_h(pack_tag([0.0; 3], 1.0, 0.0, Some(99.0))),
        Some(31.5)
    );
}
