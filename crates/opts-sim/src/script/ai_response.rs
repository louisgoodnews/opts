//! How an AI vehicle's own script turns the traffic planner's pedals into wheel torque and brake
//! force, measured by running the script (M-0095 design phase 2).
//!
//! OMSI's engine-side AI writes `Throttle` and `Brake` (and the flags `AI`, `AI_Engine`); the
//! vehicle's script computes `M_Wheel` and `Brakeforce` from them. For the shared car script
//! this is `M_Wheel = Throttle · AI_M_gas_max · (1 − Velocity / AI_v_max_erreich)` and
//! `Brakeforce = Brake · AI_F_brems_max`, but lorry, police and add-on scripts differ, so the
//! traffic model asks the script instead of assuming the law.

use super::ScriptEngine;

/// Speeds (km/h) at which the drive torque is sampled.
const SAMPLE_KMH: [f32; 8] = [0.0, 5.0, 10.0, 20.0, 30.0, 50.0, 70.0, 100.0];
/// Frames run per sample, so scripts that integrate over time settle.
const SETTLE_FRAMES: usize = 8;
/// Frame time of the measurement (s).
const DT: f32 = 1.0 / 60.0;

/// The measured response of an AI vehicle's script.
#[derive(Debug, Clone, PartialEq)]
pub struct AiResponse {
    /// `(speed km/h, M_Wheel at full throttle)`, ascending in speed.
    pub drive: Vec<(f32, f32)>,
    /// `Brakeforce` at full brake and 30 km/h, newtons.
    pub brake_force: f32,
}

impl AiResponse {
    /// Drive torque at full throttle at `kmh` (linear between samples, clamped at both ends).
    #[must_use]
    pub fn torque_at(&self, kmh: f32) -> f32 {
        let d = &self.drive;
        let Some(first) = d.first() else { return 0.0 };
        if kmh <= first.0 {
            return first.1;
        }
        for w in d.windows(2) {
            if kmh <= w[1].0 {
                let t = (kmh - w[0].0) / (w[1].0 - w[0].0).max(1e-6);
                return w[0].1 + (w[1].1 - w[0].1) * t;
            }
        }
        d.last().map_or(0.0, |p| p.1)
    }

    /// The largest drive torque over the sampled speeds (the standing-start torque for the shared
    /// law); `0` when the script gives none.
    #[must_use]
    pub fn peak_torque(&self) -> f32 {
        self.drive.iter().map(|p| p.1).fold(0.0, f32::max)
    }

    /// Whether the script produced any drive torque or brake force at all.
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.brake_force > 0.0 || self.drive.iter().any(|p| p.1 > 0.0)
    }
}

fn run(engine: &mut ScriptEngine, throttle: f32, brake: f32, kmh: f32) {
    engine.set("Throttle", throttle);
    engine.set("Brake", brake);
    engine.set("Velocity", kmh);
    engine.set("Velocity_Ground", kmh);
    // A script set with `{frame_ai}` blocks uses them for AI vehicles; the shared AI scripts
    // have only `{frame}` and tell AI driving apart by the `AI` flag.
    let ai_blocks = !engine.program.frame_ai.is_empty();
    for _ in 0..SETTLE_FRAMES {
        engine.run_frame(DT, ai_blocks);
    }
}

/// Runs `engine`'s `{init}` and `{frame}` (or `{frame_ai}`) with the AI flags set and records
/// the torque at full throttle over speed and the brake force at full brake. The engine is
/// left in an arbitrary state; measure on a fork or a dedicated engine.
pub fn measure_ai_response(engine: &mut ScriptEngine) -> AiResponse {
    engine.set("AI", 1.0);
    engine.set("AI_Engine", 1.0);
    engine.run_init();
    let drive = SAMPLE_KMH
        .iter()
        .map(|&kmh| {
            run(engine, 1.0, 0.0, kmh);
            (kmh, engine.get("M_Wheel"))
        })
        .collect();
    run(engine, 0.0, 1.0, 30.0);
    let brake_force = engine.get("Brakeforce");
    AiResponse { drive, brake_force }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHARED_CAR_SCRIPT: &str = "{frame}\n\
(L.L.Throttle) (C.L.AI_M_gas_max) * 1 (L.L.Velocity) (C.L.AI_v_max_erreich) / - * (S.L.M_Wheel)\n\
(L.L.Brake) (C.L.AI_F_brems_max) * (S.L.Brakeforce)\n{end}\n";

    fn engine() -> ScriptEngine {
        let (mut e, diags) = ScriptEngine::from_sources(&[("main_AI.osc", SHARED_CAR_SCRIPT)]);
        assert!(diags.is_empty(), "{diags:?}");
        e.consts_mut().set_scalar("AI_M_gas_max", 1000.0);
        e.consts_mut().set_scalar("AI_v_max_erreich", 190.0);
        e.consts_mut().set_scalar("AI_F_brems_max", 18_000.0);
        e
    }

    #[test]
    fn the_shared_car_script_follows_its_law() {
        let r = measure_ai_response(&mut engine());
        assert!((r.torque_at(0.0) - 1000.0).abs() < 1e-3);
        let want = 1000.0 * (1.0 - 50.0 / 190.0);
        assert!(
            (r.torque_at(50.0) - want).abs() < 0.5,
            "{}",
            r.torque_at(50.0)
        );
        assert_eq!(r.brake_force, 18_000.0);
        assert!(r.is_live());
    }

    #[test]
    fn interpolation_clamps_and_is_linear() {
        let r = AiResponse {
            drive: vec![(0.0, 100.0), (10.0, 50.0), (20.0, 0.0)],
            brake_force: 0.0,
        };
        assert_eq!(r.torque_at(-5.0), 100.0);
        assert_eq!(r.torque_at(5.0), 75.0);
        assert_eq!(r.torque_at(15.0), 25.0);
        assert_eq!(r.torque_at(99.0), 0.0);
        assert_eq!(r.peak_torque(), 100.0);
        assert!(r.is_live());
        assert!(!AiResponse {
            drive: vec![(0.0, 0.0)],
            brake_force: 0.0
        }
        .is_live());
    }
}
