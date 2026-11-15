//! Unit tests of `spacing`.

use super::*;

#[test]
fn layout_class_boundaries() {
    assert_eq!(LayoutClass::from_width(899.9), LayoutClass::Compact);
    assert_eq!(LayoutClass::from_width(900.0), LayoutClass::Medium);
    assert_eq!(LayoutClass::from_width(1400.0), LayoutClass::Medium);
    assert_eq!(LayoutClass::from_width(1400.1), LayoutClass::Expanded);
    assert_eq!(LayoutClass::from_width(1024.0), LayoutClass::Medium);
}

#[test]
fn density_scales_spacing_and_controls() {
    assert_eq!(SpacingScale::Standard.px_in(Density::Comfortable), 16.0);
    assert_eq!(SpacingScale::Standard.px_in(Density::Compact), 12.0);
    assert_eq!(SpacingScale::Standard.px_in(Density::Dense), 8.0);
    assert_eq!(SpacingScale::None.px_in(Density::Dense), 0.0);
    assert!(SpacingScale::Dense.px_in(Density::Dense) >= 2.0);
    assert_eq!(Density::Comfortable.control_height(), 40.0);
    assert_eq!(Density::Compact.control_height(), 32.0);
    assert_eq!(Density::Dense.control_height(), 24.0);
    for d in Density::ALL {
        assert_eq!(Density::parse(d.key()), Some(d));
    }
}

#[test]
fn controls_never_fall_below_the_target_size_floor_in_dense_mode_only() {
    // 32 px is the minimum target of the milestone; Dense (24 px) is the documented
    // editor-only exception.
    assert!(Density::Comfortable.control_height() >= 32.0);
    assert!(Density::Compact.control_height() >= 32.0);
}
