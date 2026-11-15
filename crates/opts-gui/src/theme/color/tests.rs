//! Unit tests of `color`.

use super::*;

#[test]
fn there_are_46_roles_in_declaration_order() {
    assert_eq!(ColorRole::all().len(), ColorRole::COUNT);
    for (i, role) in ColorRole::all().iter().enumerate() {
        assert_eq!(*role as usize, i, "{role:?}");
    }
}

#[test]
fn every_role_of_every_theme_is_opaque() {
    for scheme in [
        ColorScheme::dark(),
        ColorScheme::light(),
        ColorScheme::high_contrast(),
    ] {
        for &role in ColorRole::all() {
            assert_eq!(scheme.get(role).to_srgba().alpha, 1.0, "{role:?}");
        }
    }
}

#[test]
fn dark_is_the_default() {
    assert!(ColorScheme::default().is_dark());
    assert_eq!(ColorScheme::default().kind, ThemeKind::Dark);
}
