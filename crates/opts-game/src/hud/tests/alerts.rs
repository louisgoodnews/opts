//! Alerts, camera label, status bar, units and the developer panel.

use super::*;

#[test]
fn the_alert_row_has_a_fixed_minimum_height_and_the_alert_cannot_be_switched_off() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    let fixed = app
        .world_mut()
        .query::<(&Node, &Children)>()
        .iter(app.world())
        .any(|(n, c)| {
            n.min_height == Val::Px(44.0)
                && c.iter()
                    .any(|e| app.world().get::<build::AlertPill>(e).is_some())
        });
    assert!(fixed, "the alert line sits in a box of fixed height");
    assert!(widget::widget("alerts").is_some_and(|w| w.switch.is_none()));
}

#[test]
fn an_alert_shows_in_the_slot_and_critical_stays() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    app.world_mut().write_message(RaiseAlert {
        id: "t",
        severity: AlertSeverity::Critical,
        key: keys::ALERT_TEST,
        args: Vec::new(),
    });
    app.update();
    app.update();
    let line = |app: &mut App| {
        app.world_mut()
            .query::<(&build::HudText, &Text)>()
            .iter(app.world())
            .find(|(h, _)| h.binding == Binding::Alerts)
            .map(|(_, t)| t.0.clone())
    };
    let text = line(&mut app).unwrap_or_default();
    assert!(text.starts_with("Alert:"), "{text}");
    let visible = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&Visibility, With<build::AlertPill>>()
            .iter(app.world())
            .next()
            .copied()
    };
    assert_eq!(visible(&mut app), Some(Visibility::Inherited));
    app.world_mut().write_message(ClearAlert("t"));
    app.update();
    assert_eq!(visible(&mut app), Some(Visibility::Hidden));
}

#[test]
fn a_camera_change_shows_the_label_for_one_and_a_half_seconds() {
    let mut app = app();
    build(&mut app, VehicleKind::Bus);
    let alpha = |app: &mut App| {
        app.world_mut()
            .query::<(&FadeKind, &HudFade)>()
            .iter(app.world())
            .find(|(k, _)| **k == FadeKind::CameraLabel)
            .map(|(_, f)| f.alpha(false))
    };
    assert_eq!(alpha(&mut app), Some(0.0), "no label at the start");
    assert!(app.world().resource::<CameraLabel>().shown.is_none());
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    app.update();
    app.update();
    assert_eq!(alpha(&mut app), Some(1.0));
    // advance the fade by hand: 1.6 s later it is gone
    for (k, mut f) in app
        .world_mut()
        .query::<(&FadeKind, &mut HudFade)>()
        .iter_mut(app.world_mut())
    {
        if *k == FadeKind::CameraLabel {
            f.elapsed = CAMERA_LABEL_SECONDS + 0.1;
        }
    }
    assert_eq!(alpha(&mut app), Some(0.0));
    let text = app
        .world_mut()
        .query::<(&build::HudText, &Text)>()
        .iter(app.world())
        .find(|(h, _)| h.binding == Binding::CameraLabel)
        .map(|(_, t)| t.0.clone());
    assert_eq!(text.as_deref(), Some("Driver's view"));
}
