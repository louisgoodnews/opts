//! The ground under the units of an articulated vehicle, sampled once per fixed step.

use super::*;

/// Hands the simulation the ground under every unit of each articulated vehicle, once per fixed
/// step (design D3): the pitch, roll and height of the chassis (for the joint variables) and of
/// the following units are computed by `VehicleSimulation::apply_ground` with the fixed time
/// step, so the joint variables `articulation_<k>_beta`/`_gamma` do not depend on the frame
/// rate. The chassis' visible body motion stays in [`follow_ground`].
///
/// TEMPORARY (Q-ART-09): the heights come from the game's [`GroundIndex`]; they are replaced by
/// the M-0025 ground query when that is merged.
pub fn sample_consist_ground(
    time: Res<Time>,
    index: Res<GroundIndex>,
    mut vehicles: Query<&mut VehicleController, (With<opts_bevy::ArticulatedChain>, GroundBound)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    for mut controller in &mut vehicles {
        let sim = &mut controller.sim;
        // The height hint selects between layers (bridges): the fixed-step body of the chassis
        // once it has one, else its own elevation.
        let hint = sim
            .head_body
            .map_or(sim.chassis.elevation_z, |b| b.elevation);
        let samples: Vec<Option<GroundSample>> = sim
            .ground_probes()
            .into_iter()
            .map(|probe| {
                let p = probe.points();
                let h = |i: usize| index.height_at(p[i].x, p[i].y, hint);
                let heights = [h(0)?, h(1)?, h(2)?, h(3)?];
                let sample = GroundSample::from_heights(probe, heights);
                Some(sample)
            })
            .collect();
        sim.apply_ground(&samples, dt);
    }
}

