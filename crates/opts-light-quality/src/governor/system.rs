//! The governor's gate and the per-frame system that runs it.

use super::*;

/// Whether the game lets the governor count frames now (in a drive, not paused, not loading).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GovernorGate {
    /// Frames count only while this is true.
    pub open: bool,
}

/// Runs the governor on the real frame time (`Time<Real>`, not the simulation clock).
pub fn run_governor(
    time: Res<Time<Real>>,
    gate: Res<GovernorGate>,
    budget: Res<GovernorBudget>,
    cfg: Res<GovernorSettings>,
    input: Res<LightQualityInput>,
    mut state: ResMut<GovernorState>,
    mut last_mode: Local<LightMode>,
) {
    if *last_mode != input.mode {
        // Custom to automatic restarts at level 0; automatic to custom keeps the level.
        if input.mode == LightMode::Automatic {
            state.reset();
        }
        *last_mode = input.mode;
    }
    if input.mode != LightMode::Automatic || !gate.open || !budget.window_active {
        state.pause();
        return;
    }
    let dt = time.delta_secs();
    if let Some(c) = step(&mut state, &cfg, budget.target_ms, dt * 1000.0, dt) {
        tracing::info!(
            from = c.from,
            to = c.to,
            target_ms = c.target_ms,
            late_share = c.late_share,
            vsync = budget.vsync,
            name = level_name(c.to),
            "lighting quality level changed"
        );
    }
}
