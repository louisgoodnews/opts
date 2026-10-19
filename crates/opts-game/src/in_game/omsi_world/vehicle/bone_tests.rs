//! Tests: bone_tests.

use super::skins::is_bone_stem;

#[test]
fn bone_dummies_are_bone_underscore_letter() {
    assert!(is_bone_stem("bone_a"));
    assert!(is_bone_stem("bone_d"));
    assert!(is_bone_stem("dummya"));
    assert!(is_bone_stem("dummy_c"));
    assert!(!is_bone_stem("bone_ab"));
    assert!(!is_bone_stem("bone_1"));
    assert!(!is_bone_stem("gn_dum_gelenk_a"));
}
