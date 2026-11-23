//! The exhaust outlet of a model: OMSI names exhaust emitters with `exhaust` or the German
//! `auspuff`, and an articulated unit's exhaust is found among its other emitters (ISS-038).

use bevy::prelude::Vec3;
use opts_bevy::vehicle::exhaust_outlet;
use opts_core::formats::vehicles::model::ModelConfig;

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

#[test]
fn the_german_exhaust_name_defines_the_outlet() {
    // OMSI names the exhaust emitter's variables with `auspuff` (German for exhaust pipe).
    let m = model("[smoke]\n-1\n-5.8\n0.5\n-1\n-5.8\n0\nauspuff_a\n0.2\nauspuff_freq\n1\n");
    assert_eq!(exhaust_outlet(&m), Some(Vec3::new(-1.0, 0.5, 5.8)));
}

#[test]
fn the_exhaust_is_found_among_the_other_emitters_of_an_articulated_unit() {
    // Shape of the trailer model: an exhaust emitter first, then coolant and tyre emitters. The
    // coolant emitter has a number where the trigger name would be; it is not an exhaust.
    let m = model(
        "[smoke]\n-1\n-5.8\n0.5\n-1\n-5.8\n0\nauspuff_a\n0.2\nauspuff_freq\n1\n\n\
         [smoke]\n0.5\n-5.8\n0.4\n0\n-1\n0\n2\n0.2\nkuehl_freq\n1\n\n\
         [smoke]\n0.9\n-2.6\n0.2\n0\n-1\n0.2\n0\n0.1\ntire_freq\n1\n",
    );
    assert_eq!(exhaust_outlet(&m), Some(Vec3::new(-1.0, 0.5, 5.8)));
}
