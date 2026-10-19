//! Unit tests of the model loader.

use super::*;

/// Parts bent over a crossing base are cached apart from the flat parts and per base file.
#[test]
fn the_parts_cache_key_includes_the_crossing_base() {
    let file = Path::new("Kreuz.o3d");
    let flat = parts_key(file, &[], false, &[], None, None);
    let a = parts_key(file, &[], false, &[], None, Some(Path::new("a_base.o3d")));
    let b = parts_key(file, &[], false, &[], None, Some(Path::new("b_base.o3d")));
    assert_ne!(flat, a);
    assert_ne!(a, b);
    assert_eq!(
        a,
        parts_key(file, &[], false, &[], None, Some(Path::new("a_base.o3d")))
    );
}
