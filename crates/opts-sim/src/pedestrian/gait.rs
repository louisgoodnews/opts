//! Walking gait: joint angles from the walk phase and the `[walk_param]` of a character.
//!
//! One phase cycle (2π) is a *double step* of `stride_m` metres. The amplitudes are modelling
//! choices (OMSI's own animation is not documented): the thigh swing follows from the stride and
//! the leg length, the swing leg's knee flexes, arms swing against the legs scaled by
//! `armschwing`, the hips and the waist sway sideways by `walk_huefte` and `anim_walk_taille`.

use opts_core::formats::humans::WalkParams;

/// Joint angles (radians, positive = forward swing) and offsets of one gait sample.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Gait {
    /// Thigh swing `[left, right]`.
    pub thigh: [f32; 2],
    /// Knee flexion `[left, right]` (≥ 0, the shin folds backwards).
    pub knee: [f32; 2],
    /// Upper-arm swing `[left, right]` (opposite to the leg on the same side).
    pub arm: [f32; 2],
    /// Lateral hip offset (m, positive = towards the right).
    pub hip_shift: f32,
    /// Waist twist about the vertical axis (radians).
    pub waist_twist: f32,
    /// Vertical bob of the whole body (m, ≥ 0).
    pub bob: f32,
}

/// Maximum forward/backward thigh swing (rad) for a stride and leg length.
#[must_use]
pub fn thigh_amplitude(stride_m: f32, leg_m: f32) -> f32 {
    // Each step is half the stride: step = 2·L·sin(amplitude).
    (stride_m / (4.0 * leg_m.max(0.3))).clamp(0.0, 0.95).asin()
}

/// Gait at `phase` (radians, one cycle = one double step). `leg_m` is the hip height.
#[must_use]
pub fn gait(walk: &WalkParams, leg_m: f32, phase: f32) -> Gait {
    let a = thigh_amplitude(walk.stride_m, leg_m);
    let thigh = [a * phase.sin(), -a * phase.sin()];
    // The knee flexes while its leg swings forward (thigh velocity > 0), peaking mid-swing.
    let flex = |p: f32| 0.9 * p.cos().max(0.0).powi(2) * (p.sin().abs() + 0.35).min(1.0);
    let knee = [flex(phase), flex(phase + std::f32::consts::PI)];
    let arm_amp = 0.55 * walk.arm_swing.clamp(0.0, 1.5);
    Gait {
        thigh,
        knee,
        arm: [-arm_amp * phase.sin(), arm_amp * phase.sin()],
        hip_shift: 0.025 * walk.hip_sway * phase.sin(),
        waist_twist: 0.12 * walk.waist_sway * phase.sin() + 0.06 * phase.sin(),
        bob: 0.02 * (1.0 - (2.0 * phase).cos()) * 0.5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walk() -> WalkParams {
        WalkParams {
            stride_m: 1.4,
            upper_arm_beta_deg: 70.0,
            arm_swing: 0.7,
            hip_sway: 1.0,
            waist_sway: 0.0,
        }
    }

    #[test]
    fn legs_and_arms_alternate() {
        let g = gait(&walk(), 0.9, 1.0);
        assert!((g.thigh[0] + g.thigh[1]).abs() < 1e-6);
        assert!((g.arm[0] + g.arm[1]).abs() < 1e-6);
        // Left arm swings against the left leg.
        assert!(g.thigh[0] * g.arm[0] < 0.0);
    }

    #[test]
    fn step_length_matches_the_stride() {
        let a = thigh_amplitude(1.4, 0.9);
        // Heel-to-heel distance of the extreme positions is one step = half the stride.
        let step = 2.0 * 0.9 * a.sin();
        assert!((step - 0.7).abs() < 1e-4);
    }

    #[test]
    fn one_cycle_is_periodic() {
        let a = gait(&walk(), 0.9, 0.7);
        let b = gait(&walk(), 0.9, 0.7 + std::f32::consts::TAU);
        assert!((a.thigh[0] - b.thigh[0]).abs() < 1e-4);
        assert!((a.knee[1] - b.knee[1]).abs() < 1e-4);
    }

    #[test]
    fn knees_never_hyperextend() {
        for k in 0..100 {
            let g = gait(&walk(), 0.9, k as f32 * 0.1);
            assert!(g.knee.iter().all(|x| *x >= 0.0 && *x <= 1.0));
            assert!(g.bob >= 0.0);
        }
    }
}
