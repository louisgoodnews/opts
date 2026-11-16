//! The cab handover: the takeover of the cab scripts and the handover system.

use super::*;

/// A hand-over in progress: the old cab was put in neutral, and the new one takes over once the
/// control cables have carried that to it (the NF6D's second cab deactivates itself when it still
/// sees the first one active).
#[derive(Component, Debug, Clone, Copy)]
pub struct CabHandover {
    /// The cab that takes over.
    pub to: Option<Entity>,
    /// Seconds left before the next step.
    pub left: f32,
    /// `0`: the new cab claims the controls (`kw_s_N`), then the old one learns of it through the
    /// cables and deactivates; `1`: the new cab goes into drive (`automatic_D`).
    pub step: u8,
}

/// Seconds between putting the old cab in neutral and the new cab taking over.
pub(super) const HANDOVER_S: f32 = 0.3;
/// Seconds between the new cab's claim and its drive position.
pub(super) const DRIVE_AFTER_S: f32 = 1.2;

/// Hands a cab over. The cab that gives up presses `kw_s_R` (the NF6D's "deactivate this cab":
/// it marks itself inactive, which the other cab reads through the cables); the cab that takes
/// over presses `kw_s_N` and `automatic_D`, which work only while the other is inactive.
pub(super) fn hand_over(engine: &mut ScriptEngine, take_over: bool) {
    // The claim and the drive position are pressed one after the other (`cab_handover_system`).
    if !take_over && engine.has_trigger("kw_s_r") {
        engine.trigger("kw_s_r");
    }
}

/// Completes a hand-over: the new cab takes the controls.
pub fn cab_handover_system(
    time: Res<Time>,
    mut commands: Commands,
    mut players: Query<(Entity, &mut CabHandover, &VehicleScripts, &mut RailControls)>,
    parts: Query<&VehicleScripts, With<ScriptPart>>,
) {
    for (entity, mut h, head, mut controls) in &mut players {
        h.left -= time.delta_secs();
        if h.left > 0.0 {
            continue;
        }
        let to = ActiveCab(h.to);
        if h.step == 0 {
            if let Ok(mut new) = cab_scripts(head, Some(to), &parts).engine.lock() {
                if new.has_trigger("kw_s_n") {
                    new.trigger("kw_s_n");
                }
            }
            h.step = 1;
            h.left = DRIVE_AFTER_S;
            continue;
        }
        if let Ok(mut new) = cab_scripts(head, Some(to), &parts).engine.lock() {
            if new.has_trigger("automatic_d") {
                new.trigger("automatic_d");
            }
        }
        controls.lever = 0.0;
        controls.select(1);
        commands
            .entity(entity)
            .insert((to, CabTrace(1.5)))
            .remove::<CabHandover>();
        tracing::info!(cab = ?h.to, "active cab took over");
    }
}
