//! Unit tests of `mod.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;
use crate::vehicle::inputs::GearPosition;

mod grip;
mod ground;
mod loads;
mod roll;
mod stepping;

/// A bus with one unit behind it: hitch 1.5 m behind the chassis origin, front coupling
/// 1 m ahead of the unit's origin, rear axle 6 m behind that.
fn articulated() -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    sim.attach_trailer(DVec2::new(0.0, -1.5), DVec2::new(0.0, 1.0), 7.0, 52.5, None);
    sim.inputs.set_parking_brake(false);
    sim.inputs.set_gear(GearPosition::Drive);
    sim.inputs.set_throttle(0.4);
    sim.inputs.set_steering(0.5);
    sim
}

fn unit(mass_kg: f32, axles: &[(f32, bool)]) -> UnitLoad {
    UnitLoad {
        mass_kg,
        axles: axles
            .iter()
            .map(|&(y, is_driven)| UnitAxle {
                longitudinal_m: y,
                is_driven,
                rated_n: consist::DEFAULT_RATED_AXLE_N,
            })
            .collect(),
    }
}

/// A bus of 11.5 t with a trailer of `trailer_kg` (none: solo), on a straight line.
fn towing(trailer_kg: Option<f32>, trailer_driven: bool) -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    if let Some(m) = trailer_kg {
        let load = unit(m, &[(-4.0, trailer_driven)]);
        sim.attach_trailer(
            DVec2::new(0.0, -2.0),
            DVec2::new(0.0, 1.0),
            6.0,
            52.5,
            Some(load),
        );
    }
    sim.inputs.set_parking_brake(false);
    sim
}

/// Samples a plane `z = slope · y` under every probe.
fn samples_on_slope(sim: &VehicleSimulation, slope: f32) -> Vec<Option<GroundSample>> {
    sim.ground_probes()
        .into_iter()
        .map(|probe| {
            let h = probe.points().map(|p| p.y * slope);
            Some(GroundSample::from_heights(probe, h))
        })
        .collect()
}
