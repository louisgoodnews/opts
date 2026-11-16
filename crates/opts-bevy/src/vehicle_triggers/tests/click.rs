//! Scripted click tests: press, drag over frames and release of a knob.

use super::*;

const KNOB: &str = "{trigger:knob}\n 1 (S.L.pressed)\n{end}\n\
{trigger:knob_drag}\n (L.S.mouse_y) -75 / (L.L.rot) + 0 max 1 min (S.L.rot) (L.L.drags) 1 + (S.L.drags)\n{end}\n\
{trigger:knob_off}\n 1 (S.L.released)\n{end}\n";

fn click_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<ScriptedClicks>()
        .add_systems(Update, scripted_click_system);
    let (engine, diags) = opts_sim::script::ScriptEngine::from_sources(&[("knob.osc", KNOB)]);
    assert!(
        diags
            .iter()
            .all(|d| d.severity != opts_sim::script::Severity::Error),
        "{diags:?}"
    );
    let e = app
        .world_mut()
        .spawn((PlayerVehicle, VehicleScripts::new(engine)))
        .id();
    (app, e)
}

fn var(app: &App, e: Entity, name: &str) -> f32 {
    app.world()
        .get::<VehicleScripts>(e)
        .and_then(|s| s.engine.lock().ok().map(|g| g.get(name)))
        .unwrap_or(f32::NAN)
}

fn frames(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.update();
    }
}

#[test]
fn a_scripted_click_presses_drags_over_frames_and_releases() {
    let (mut app, e) = click_app();
    app.world_mut()
        .resource_mut::<ScriptedClicks>()
        .0
        .push(ScriptedClick::new("knob", Vec2::new(0.0, -60.0), 6, 0.0));
    frames(&mut app, 1);
    assert_eq!(var(&app, e, "pressed"), 1.0);
    assert_eq!(var(&app, e, "rot"), 0.0);
    frames(&mut app, 6);
    // The drag trigger ran once per frame: six calls, each adding 10 / 75.
    assert_eq!(var(&app, e, "drags"), 6.0);
    assert!(
        (var(&app, e, "rot") - 0.8).abs() < 1e-4,
        "rot {}",
        var(&app, e, "rot")
    );
    assert_eq!(var(&app, e, "released"), 0.0, "still held");
    frames(&mut app, 1);
    assert_eq!(var(&app, e, "released"), 1.0);
    assert!(app.world().resource::<ScriptedClicks>().0.is_empty());
    // The mouse values are back at 0 after the release.
    assert_eq!(var(&app, e, "mouse_y"), 0.0);
}

#[test]
fn a_click_without_a_drag_is_a_press_and_a_release() {
    let (mut app, e) = click_app();
    app.world_mut()
        .resource_mut::<ScriptedClicks>()
        .0
        .push(ScriptedClick::new("knob", Vec2::ZERO, 0, 0.0));
    frames(&mut app, 3);
    assert_eq!(
        (
            var(&app, e, "pressed"),
            var(&app, e, "released"),
            var(&app, e, "drags")
        ),
        (1.0, 1.0, 0.0)
    );
}
