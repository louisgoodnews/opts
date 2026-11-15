//! The systems that feed the alert queue: the queue update and the parking brake alert.

use super::*;

/// Applies raise and clear messages and ages the queue (virtual time: a paused game keeps its
/// alerts).
pub fn update_queue(
    time: Res<Time>,
    mut raise: MessageReader<RaiseAlert>,
    mut clear: MessageReader<ClearAlert>,
    mut queue: ResMut<AlertQueue>,
) {
    for r in raise.read() {
        queue.raise(r.clone());
    }
    for ClearAlert(id) in clear.read() {
        queue.clear(id);
    }
    if !queue.active.is_empty() {
        queue.tick(time.delta_secs());
    }
}

/// The parking brake held on while the vehicle moves.
pub fn parking_brake_alert(
    values: Res<HudValues>,
    mut raise: MessageWriter<RaiseAlert>,
    mut clear: MessageWriter<ClearAlert>,
    mut last: Local<Option<AlertSeverity>>,
) {
    const ID: &str = "brake.moving";
    let now = (values.has_vehicle && values.parking_brake && values.speed_kmh > BRAKE_WARN_KMH)
        .then(|| {
            if values.speed_kmh > BRAKE_CRITICAL_KMH {
                AlertSeverity::Critical
            } else {
                AlertSeverity::Warning
            }
        });
    if now == *last {
        return;
    }
    match now {
        Some(severity) => {
            raise.write(RaiseAlert {
                id: ID,
                severity,
                key: keys::ALERT_BRAKE_MOVING,
                args: Vec::new(),
            });
        }
        None => {
            clear.write(ClearAlert(ID));
        }
    }
    *last = now;
}
