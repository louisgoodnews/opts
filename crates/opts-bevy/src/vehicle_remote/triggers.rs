//! Script triggers raised by the remote controls: at once, and their `_off` after the hold.

use super::*;

/// Raises script trigger `name` on the player's vehicle; returns whether a script handled it, or
/// `None` when there is no vehicle with scripts.
pub fn raise_trigger(world: &mut World, name: &str) -> Option<bool> {
    let mut q = world.query_filtered::<&VehicleScripts, With<PlayerVehicle>>();
    let scripts = q.iter(world).next()?;
    let mut engine = scripts.engine.lock().ok()?;
    let handled = engine.trigger(name);
    tracing::debug!(trigger = name, handled, "vehicle script trigger (remote)");
    Some(handled)
}

/// Raises the scheduled `_off` triggers whose time has come.
pub fn release_trigger_system(
    time: Res<Time>,
    mut remote: ResMut<RemoteControls>,
    vehicles: Query<&VehicleScripts, With<PlayerVehicle>>,
) {
    if remote.release_triggers.is_empty() {
        return;
    }
    let dt = time.delta_secs();
    let mut due = Vec::new();
    remote.release_triggers.retain_mut(|(name, left)| {
        *left -= dt;
        if *left <= 0.0 {
            due.push(std::mem::take(name));
            false
        } else {
            true
        }
    });
    for name in due {
        for scripts in &vehicles {
            if let Ok(mut engine) = scripts.engine.lock() {
                let handled = engine.trigger(&name);
                tracing::debug!(trigger = %name, handled, "vehicle script trigger (remote release)");
            }
        }
    }
}
