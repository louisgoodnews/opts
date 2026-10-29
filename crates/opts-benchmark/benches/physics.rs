//! Criterion benchmark of the vehicle physics step (M-0028): one `VehicleSimulation::step` of a
//! standard bus at the default 64 Hz tick, at rest, accelerating and cruising with a steering input.
//!
//! Run with `cargo bench -p opts-benchmark --features physics --bench physics`.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use opts_sim::vehicle::inputs::GearPosition;
use opts_sim::vehicle::VehicleSimulation;

/// The fixed tick of the game (64 Hz).
const DT: f32 = 1.0 / 64.0;

/// A standard bus in gear with the parking brake released.
fn bus() -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    sim.inputs.set_parking_brake(false);
    sim.inputs.set_gear(GearPosition::Drive);
    sim
}

fn physics(c: &mut Criterion) {
    let mut group = c.benchmark_group("vehicle_step");
    group.bench_function("at_rest", |b| {
        let mut sim = bus();
        b.iter(|| sim.step(black_box(DT)));
    });
    group.bench_function("accelerating", |b| {
        let mut sim = bus();
        sim.inputs.set_throttle(0.8);
        b.iter(|| sim.step(black_box(DT)));
    });
    group.bench_function("cruising_in_a_curve", |b| {
        let mut sim = bus();
        sim.inputs.set_throttle(0.5);
        sim.inputs.set_steering(0.2);
        // Reach speed first, so the step is measured at its usual cost.
        for _ in 0..(20 * 64) {
            sim.step(DT);
        }
        b.iter(|| sim.step(black_box(DT)));
    });
    group.finish();
}

criterion_group!(benches, physics);
criterion_main!(benches);
