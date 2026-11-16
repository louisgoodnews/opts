//! The cab trace: how long the player has been in the cab, and the trace system.

use super::*;


/// Debug: traces the cab flags of the new cab for a while after it took over.
#[derive(Component, Debug, Clone, Copy)]
pub struct CabTrace(pub f32);

/// Logs the flags of the active cab for the first seconds after a hand-over (debug level).
pub fn cab_trace_system(
    time: Res<Time>,
    mut commands: Commands,
    mut players: Query<(Entity, &mut CabTrace, &VehicleScripts, Option<&ActiveCab>)>,
    parts: Query<&VehicleScripts, With<ScriptPart>>,
) {
    for (entity, mut t, head, active) in &mut players {
        t.0 -= time.delta_secs();
        if t.0 <= 0.0 {
            commands.entity(entity).remove::<CabTrace>();
            continue;
        }
        if let Ok(e) = cab_scripts(head, active.copied(), &parts).engine.lock() {
            let g = |n: &str| (e.get(n) * 100.0).round() / 100.0;
            tracing::debug!(
                drive = g("b_kopf_drive"),
                neutral = g("b_kopf_neutral"),
                slw_b = g("slw_b_kopf_inaktiv"),
                slr_a = g("slr_a_kopf_inaktiv"),
                slr_b = g("slr_b_kopf_inaktiv"),
                bat = g("slar_elec_bat"),
                a_drive = g("a_kopf_drive"),
                "cab trace"
            );
        }
    }
}
