use super::unit_size;

#[test]
fn a_bicycle_keeps_its_length() {
    assert_eq!(unit_size(-1.0, 0.76, None), (1.76, 1.0));
    assert_eq!(
        unit_size(f32::INFINITY, f32::NEG_INFINITY, Some(1.76)),
        (1.76, 0.88)
    );
    assert_eq!(
        unit_size(-0.1, 0.1, None),
        (4.4, 2.2),
        "a speck is no vehicle"
    );
    assert_eq!(unit_size(-40.0, 0.0, Some(30.0)), (4.4, 2.2));
}
