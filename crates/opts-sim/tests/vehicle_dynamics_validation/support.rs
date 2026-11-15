//! Shared fixtures of the M-0073 validation: the two-axle file, the articulated bus and the runs.

use glam::Vec2;
use opts_core::formats::vehicles::bus::BusManifest;
use opts_sim::vehicle::inputs::GearPosition;
use opts_sim::{UnitLoad, VehicleSimulation};

pub(super) const KMH_50: f32 = 50.0 / 3.6;

/// A two-axle vehicle file: mass in t, centre of gravity, wheelbase 2 x `half`, axles rated for
/// `rated_kn` each.
pub(super) fn file(mass_t: f32, cg: f32, half: f32, rated_kn: f32) -> BusManifest {
    BusManifest::parse(&format!(
        "[mass]\n{mass_t}\n0\n0\n0\n\n[schwerpunkt]\n{cg}\n\n\
         [newachse]\nachse_long\n{half}\nachse_raddurchmesser\n0.7\nachse_feder\n150\n\
         achse_maxforce\n{rated_kn}\nachse_daempfer\n12\n\n\
         [newachse]\nachse_long\n-{half}\nachse_raddurchmesser\n0.7\nachse_feder\n150\n\
         achse_maxforce\n{rated_kn}\nachse_daempfer\n12\nachse_antrieb\n1\n"
    ))
    .expect("parses")
}

pub(super) fn at_speed(mut sim: VehicleSimulation, v: f32) -> VehicleSimulation {
    sim.inputs.set_parking_brake(false);
    sim.inputs.set_gear(GearPosition::Neutral);
    sim.chassis.local_velocity = Vec2::new(0.0, v);
    sim
}

/// Distance (m) to a standstill from `v` with the service brake fully applied, stepping `dt`.
pub(super) fn stopping_distance(mut sim: VehicleSimulation, dt: f32) -> f32 {
    let start = sim.chassis.position.y;
    sim.inputs.set_service_brake(1.0);
    for _ in 0..(20.0 / dt) as usize {
        sim.step(dt);
        if sim.chassis.forward_speed_m_s() == 0.0 {
            break;
        }
    }
    assert_eq!(sim.chassis.forward_speed_m_s(), 0.0, "must come to rest");
    sim.chassis.position.y - start
}

/// A two-unit bus: head and trailer of 14 t each, four axles rated for 80 kN each.
pub(super) fn articulated(loaded_extra_kg: f32) -> VehicleSimulation {
    let mut sim = VehicleSimulation::from_manifest(&file(14.0, 1.2, 3.0, 80.0));
    sim.chassis.total_mass_kg += loaded_extra_kg;
    let mut trailer = UnitLoad::with_axles(14_000.0 + loaded_extra_kg, 2, 0);
    for a in &mut trailer.axles {
        a.rated_n = 80_000.0;
    }
    sim.set_trailers(vec![trailer]);
    sim
}
