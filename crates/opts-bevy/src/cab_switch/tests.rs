//! Tests of the cab switch: the next cab wraps around, and a single cab has no other.

use super::*;

#[test]
fn the_next_cab_wraps_and_a_single_cab_has_no_other() {
    let a = Some(Entity::from_raw_u32(1).unwrap());
    let b = Some(Entity::from_raw_u32(2).unwrap());
    assert_eq!(next_cab(&[None], None), None);
    assert_eq!(next_cab(&[None, a], None), Some(a));
    assert_eq!(next_cab(&[None, a], a), Some(None));
    assert_eq!(next_cab(&[None, a, b], a), Some(b));
    assert_eq!(next_cab(&[None, a, b], b), Some(None));
    // The current cab is not in the list any more: start from the first.
    assert_eq!(next_cab(&[a, b], None), Some(a));
}
