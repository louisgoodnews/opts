//! The alert slot (UI-P13): one alert at a time, short, at the cluster, never modal.
//!
//! Sources write [`RaiseAlert`] and [`ClearAlert`]. The queue shows the most severe, newest alert;
//! *critical* alerts stay until their source clears them, the others leave after
//! [`AUTO_SECONDS`]. The slot has a fixed height, so an alert appearing moves nothing.

use bevy::prelude::*;

use super::keys;
use super::values::HudValues;

/// Seconds a non-critical alert stays.
pub const AUTO_SECONDS: f32 = 6.0;
/// Speed above which a parking brake in use is critical, km/h.
pub const BRAKE_CRITICAL_KMH: f32 = 15.0;
/// Speed above which a parking brake in use is worth a warning, km/h.
pub const BRAKE_WARN_KMH: f32 = 3.0;

/// How serious an alert is; decides the colour, the word, and whether it stays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    /// Something worth knowing; leaves by itself.
    Info,
    /// Something to act on soon; leaves by itself.
    Warning,
    /// Something to act on now; stays until its source clears it.
    Critical,
}

impl AlertSeverity {
    /// Message key of the word shown before the text (colour is never the only signal).
    #[must_use]
    pub const fn word_key(self) -> &'static str {
        match self {
            Self::Info => keys::ALERT_INFO,
            Self::Warning => keys::ALERT_WARNING,
            Self::Critical => keys::ALERT_CRITICAL,
        }
    }

    /// Whether the alert stays until cleared.
    #[must_use]
    pub const fn sticky(self) -> bool {
        matches!(self, Self::Critical)
    }

    /// Stable id (control protocol).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }

    /// Parses an [`id`](Self::id).
    #[must_use]
    pub fn parse(id: &str) -> Option<Self> {
        [Self::Info, Self::Warning, Self::Critical]
            .into_iter()
            .find(|s| s.id() == id)
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn raise(q: &mut AlertQueue, id: &'static str, severity: AlertSeverity) {
        q.raise(RaiseAlert {
            id,
            severity,
            key: keys::ALERT_BRAKE_MOVING,
            args: Vec::new(),
        });
    }

    #[test]
    fn one_alert_is_visible_and_the_most_severe_wins() {
        let mut q = AlertQueue::default();
        raise(&mut q, "a", AlertSeverity::Warning);
        raise(&mut q, "b", AlertSeverity::Critical);
        raise(&mut q, "c", AlertSeverity::Info);
        assert_eq!(q.visible().map(|a| a.id), Some("b"));
        raise(&mut q, "d", AlertSeverity::Critical);
        assert_eq!(q.visible().map(|a| a.id), Some("d"), "newest of equals");
    }

    #[test]
    fn critical_alerts_stay_and_the_rest_leave() {
        let mut q = AlertQueue::default();
        raise(&mut q, "w", AlertSeverity::Warning);
        raise(&mut q, "c", AlertSeverity::Critical);
        q.tick(AUTO_SECONDS + 1.0);
        assert_eq!(q.active.len(), 1);
        assert_eq!(q.visible().map(|a| a.id), Some("c"));
        q.tick(600.0);
        assert!(q.visible().is_some(), "sticky until cleared");
        q.clear("c");
        assert!(q.visible().is_none());
    }

    #[test]
    fn the_same_source_replaces_its_alert() {
        let mut q = AlertQueue::default();
        raise(&mut q, "x", AlertSeverity::Warning);
        raise(&mut q, "x", AlertSeverity::Critical);
        assert_eq!(q.active.len(), 1);
        assert_eq!(
            q.visible().map(|a| a.severity),
            Some(AlertSeverity::Critical)
        );
    }

    #[test]
    fn the_parking_brake_alert_follows_speed_and_clears() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .add_message::<RaiseAlert>()
            .add_message::<ClearAlert>()
            .init_resource::<AlertQueue>()
            .insert_resource(HudValues {
                has_vehicle: true,
                parking_brake: true,
                speed_kmh: 5.0,
                ..HudValues::default()
            })
            .add_systems(Update, (parking_brake_alert, update_queue).chain());
        app.update();
        let sev = |app: &App| {
            app.world()
                .resource::<AlertQueue>()
                .visible()
                .map(|a| a.severity)
        };
        assert_eq!(sev(&app), Some(AlertSeverity::Warning));
        app.world_mut().resource_mut::<HudValues>().speed_kmh = 30.0;
        app.update();
        assert_eq!(sev(&app), Some(AlertSeverity::Critical));
        app.world_mut().resource_mut::<HudValues>().parking_brake = false;
        app.update();
        assert_eq!(sev(&app), None);
    }
}
