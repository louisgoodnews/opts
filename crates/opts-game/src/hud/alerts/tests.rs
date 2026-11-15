//! Tests of the alert queue: severity order, staying and leaving, replacement and the parking brake.

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
