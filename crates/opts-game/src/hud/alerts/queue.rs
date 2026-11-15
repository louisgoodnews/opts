//! The alert messages and the queue: raising, clearing, ageing and the visible alert.

use super::*;

/// Asks for an alert. The same `id` again replaces the earlier one (new severity, new timer).
#[derive(Message, Debug, Clone)]
pub struct RaiseAlert {
    /// Identifies the source (`brake.moving`).
    pub id: &'static str,
    /// Severity.
    pub severity: AlertSeverity,
    /// Message key of the text.
    pub key: &'static str,
    /// Variables of the text.
    pub args: Vec<(&'static str, String)>,
}

/// Withdraws the alert of `id`.
#[derive(Message, Debug, Clone, Copy)]
pub struct ClearAlert(pub &'static str);

/// An alert that is waiting or showing.
#[derive(Debug, Clone)]
pub struct ActiveAlert {
    /// Source id.
    pub id: &'static str,
    /// Severity.
    pub severity: AlertSeverity,
    /// Message key.
    pub key: &'static str,
    /// Variables.
    pub args: Vec<(&'static str, String)>,
    /// Seconds since it was raised.
    pub age: f32,
    /// Order of raising (newest highest).
    pub seq: u64,
}

/// All live alerts.
#[derive(Resource, Debug, Default)]
pub struct AlertQueue {
    /// The alerts, in no particular order.
    pub active: Vec<ActiveAlert>,
    next_seq: u64,
}

impl AlertQueue {
    /// Adds or replaces an alert.
    pub fn raise(&mut self, r: RaiseAlert) {
        self.active.retain(|a| a.id != r.id);
        self.next_seq += 1;
        self.active.push(ActiveAlert {
            id: r.id,
            severity: r.severity,
            key: r.key,
            args: r.args,
            age: 0.0,
            seq: self.next_seq,
        });
    }

    /// Removes the alert of `id`.
    pub fn clear(&mut self, id: &str) {
        self.active.retain(|a| a.id != id);
    }

    /// Ages the alerts and drops the expired non-critical ones.
    pub fn tick(&mut self, dt: f32) {
        for a in &mut self.active {
            a.age += dt;
        }
        self.active
            .retain(|a| a.severity.sticky() || a.age < AUTO_SECONDS);
    }

    /// The alert to show: the most severe, then the newest.
    #[must_use]
    pub fn visible(&self) -> Option<&ActiveAlert> {
        self.active.iter().max_by_key(|a| (a.severity, a.seq))
    }
}
