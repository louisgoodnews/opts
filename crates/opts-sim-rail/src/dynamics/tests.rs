//! Unit tests for `super` (`dynamics.rs`).

use super::*;

fn go() -> RailInput {
    RailInput {
        traction: 1.0,
        direction: 1,
        ..Default::default()
    }
}

pub(super) fn freight() -> (RailDynamics, RailDynamics) {
    // One 84 t locomotive and twenty 40 t wagons (884 t): what a long `.zug` is.
    let train = RailDynamics::for_consist(ConsistLoad {
        total_kg: 884_000.0,
        powered_kg: 84_000.0,
        braked_kg: 884_000.0,
    });
    (train, RailDynamics::tram(84_000.0))
}

/// The force that produced `a` on `d` at speed `v`: mass times acceleration plus resistance.
pub(super) fn pull_n(d: &RailDynamics, a: f32, v: f32) -> f32 {
    a * d.mass_kg + d.davis_a + d.davis_b * v + d.davis_c * v * v
}

mod drive;
mod mass;
