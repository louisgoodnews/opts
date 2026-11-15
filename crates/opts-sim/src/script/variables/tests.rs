//! Unit tests for `super` (`variables.rs`).
use super::*;

#[test]
fn interner_is_case_insensitive() {
    let mut i = Interner::default();
    let a = i.intern("GetTime");
    assert_eq!(i.intern("gettime"), a);
    assert_ne!(i.intern("other"), a);
    assert_eq!(i.get("GETTIME"), Some(a));
    assert_eq!(i.name(a), "gettime");
}

#[test]
fn unwritten_variables_read_zero_and_empty() {
    let mut v = VariableStore::default();
    assert_eq!(v.float(5), 0.0);
    v.set_float(5, 2.5);
    v.set_string(2, "ab".into());
    assert_eq!((v.float(5), v.string(2), v.string(9)), (2.5, "ab", ""));
}

#[test]
fn curve_interpolates_and_clamps() {
    let c = Curve::new(vec![(10.0, 100.0), (0.0, 0.0)]);
    assert_eq!(c.eval(-5.0), 0.0);
    assert_eq!(c.eval(5.0), 50.0);
    assert_eq!(c.eval(99.0), 100.0);
    assert_eq!(Curve::default().eval(1.0), 0.0);
}
