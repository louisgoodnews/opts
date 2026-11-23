//! Unit tests for `super` (`exhaust.rs`): the outlet comes from a defined exhaust emitter only.

use super::*;

fn model(smoke_blocks: &str) -> ModelConfig {
    ModelConfig::parse(smoke_blocks).expect("model parses")
}

#[test]
fn outlet_is_the_exhaust_emitter_position_in_bevy_axes() {
    let m = model("[smoke]\n0.5\n-5.5\n1.2\n0\n1\n0\nmotor_on\n1\nexhaust_freq\n");
    // OMSI (x, y, z) = (0.5, -5.5, 1.2) becomes Bevy (x, z, -y).
    assert_eq!(exhaust_outlet(&m), Some(Vec3::new(0.5, 1.2, 5.5)));
}

#[test]
fn a_non_exhaust_emitter_does_not_define_the_outlet() {
    let m = model("[smoke]\n0\n0\n1\n0\n0\n1\nwheel_spray\n1\nwheel_freq\n");
    assert_eq!(exhaust_outlet(&m), None);
}

#[test]
fn a_model_without_smoke_defines_no_outlet() {
    assert_eq!(exhaust_outlet(&model("[mesh]\na.o3d\n")), None);
}

#[test]
fn the_first_exhaust_emitter_wins_over_later_ones() {
    let m = model(
        "[smoke]\n1\n0\n0\n0\n0\n1\nEXHAUST_a\n1\n\n[smoke]\n2\n0\n0\n0\n0\n1\nexhaust_b\n1\n\n",
    );
    assert_eq!(exhaust_outlet(&m), Some(Vec3::new(1.0, 0.0, 0.0)));
}
