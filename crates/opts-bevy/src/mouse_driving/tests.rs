use bevy::ecs::system::RunSystemOnce;

use super::*;

const SIZE: Vec2 = Vec2::new(1000.0, 600.0);

#[test]
fn the_middle_is_neither_pedal_and_straight() {
    assert_eq!(axes_of(Vec2::new(500.0, 300.0), SIZE), (0.0, 0.0, 0.0));
    // Inside the dead zone nothing moves.
    let (t, b, s) = axes_of(Vec2::new(510.0, 305.0), SIZE);
    assert_eq!((t, b, s), (0.0, 0.0, 0.0));
}

#[test]
fn height_is_throttle_above_the_middle_and_brake_below() {
    let (t, b, _) = axes_of(Vec2::new(500.0, 0.0), SIZE);
    assert_eq!((t, b), (1.0, 0.0), "top edge: full throttle");
    let (t, b, _) = axes_of(Vec2::new(500.0, 600.0), SIZE);
    assert_eq!((t, b), (0.0, 1.0), "bottom edge: full brake");
    let (t, _, _) = axes_of(Vec2::new(500.0, 150.0), SIZE);
    assert!(
        (t - 0.5 / 0.96 + 0.04 / 0.96).abs() < 0.2 && (0.4..0.6).contains(&t),
        "a quarter up: {t}"
    );
}

#[test]
fn width_is_steering_with_full_lock_at_the_edges() {
    let (_, _, s) = axes_of(Vec2::new(0.0, 300.0), SIZE);
    assert_eq!(s, -1.0);
    let (_, _, s) = axes_of(Vec2::new(1000.0, 300.0), SIZE);
    assert_eq!(s, 1.0);
    let (_, _, s) = axes_of(Vec2::new(750.0, 300.0), SIZE);
    assert!((0.4..0.6).contains(&s), "{s}");
}

#[test]
fn a_cursor_outside_the_window_is_clamped() {
    let (t, _, s) = axes_of(Vec2::new(-400.0, -300.0), SIZE);
    assert_eq!((t, s), (1.0, -1.0));
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<MouseDriving>()
        .add_systems(Update, mouse_driving_buttons);
    app
}

fn click(app: &mut App, button: MouseButton) {
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(button);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(button);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
}

#[test]
fn the_wheel_press_switches_the_mode_and_clears_the_freed_state() {
    let mut app = app();
    click(&mut app, MouseButton::Middle);
    assert!(app.world().resource::<MouseDriving>().on);
    app.world_mut().resource_mut::<MouseDriving>().freed = true;
    click(&mut app, MouseButton::Middle);
    assert!(!app.world().resource::<MouseDriving>().on);
    click(&mut app, MouseButton::Middle);
    let s = *app.world().resource::<MouseDriving>();
    assert!(s.on && !s.freed && s.drives());
}

#[test]
fn a_disabled_mode_ignores_the_wheel_and_switches_off() {
    let mut app = app();
    click(&mut app, MouseButton::Middle);
    app.world_mut().resource_mut::<MouseDriving>().enabled = false;
    click(&mut app, MouseButton::Middle);
    let s = *app.world().resource::<MouseDriving>();
    assert!(!s.on && !s.drives());
}

fn drive_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<MouseDriving>()
        .add_systems(Update, mouse_driving_axes);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    let vehicle = app
        .world_mut()
        .spawn((
            VehicleController::standard_bus(12_000.0, 5.9),
            PlayerVehicle,
        ))
        .id();
    (app, vehicle)
}

fn put_cursor(app: &mut App, at: Option<Vec2>) {
    let mut q = app.world_mut().query::<&mut Window>();
    for mut w in q.iter_mut(app.world_mut()) {
        w.set_cursor_position(at);
    }
}

fn pedals(app: &App, vehicle: Entity) -> (f32, f32, f32) {
    let c = app.world().get::<VehicleController>(vehicle).unwrap();
    (
        c.inputs().throttle(),
        c.inputs().service_brake(),
        c.inputs().steering(),
    )
}

#[test]
fn the_cursor_drives_while_the_mode_is_on_and_lets_go_when_freed() {
    let (mut app, v) = drive_app();
    let (w, h) = {
        let mut q = app.world_mut().query::<&Window>();
        let win = q.single(app.world()).unwrap();
        (win.width(), win.height())
    };
    // Mode off: the cursor does nothing.
    put_cursor(&mut app, Some(Vec2::new(w * 0.5, 0.0)));
    app.update();
    assert_eq!(pedals(&app, v), (0.0, 0.0, 0.0));
    // On: top middle is full throttle, straight.
    app.world_mut().resource_mut::<MouseDriving>().on = true;
    app.update();
    assert_eq!(pedals(&app, v), (1.0, 0.0, 0.0));
    assert!(app.world().resource::<MouseDriving>().steering);
    // Bottom right: brake and right lock.
    put_cursor(&mut app, Some(Vec2::new(w - 0.5, h - 0.5)));
    app.update();
    let (t, b, s) = pedals(&app, v);
    assert!(t == 0.0 && b > 0.99 && s > 0.99, "{:?}", (t, b, s));
    // Freed: the pedals are let go and the cursor is its own again.
    app.world_mut().resource_mut::<MouseDriving>().freed = true;
    app.update();
    assert_eq!(pedals(&app, v), (0.0, 0.0, 0.0));
    assert!(!app.world().resource::<MouseDriving>().steering);
    put_cursor(&mut app, Some(Vec2::new(0.0, 0.0)));
    app.update();
    assert_eq!(
        pedals(&app, v),
        (0.0, 0.0, 0.0),
        "a freed mouse drives nothing"
    );
}

#[test]
fn a_cursor_outside_the_window_does_not_drive() {
    let (mut app, v) = drive_app();
    app.world_mut().resource_mut::<MouseDriving>().on = true;
    put_cursor(&mut app, None);
    app.update();
    assert_eq!(pedals(&app, v), (0.0, 0.0, 0.0));
    assert!(!app.world().resource::<MouseDriving>().steering);
}

#[test]
fn closing_the_input_gate_releases_what_the_mouse_held() {
    let (mut app, v) = drive_app();
    app.add_systems(
        Update,
        mouse_driving_gate_closed
            .after(mouse_driving_axes)
            .run_if(|| false),
    );
    app.world_mut().resource_mut::<MouseDriving>().on = true;
    put_cursor(&mut app, Some(Vec2::new(500.0, 0.0)));
    app.update();
    assert_eq!(pedals(&app, v).0, 1.0);
    // The gate system runs on its own condition: call it as the schedule would when closed.
    app.world_mut()
        .run_system_once(mouse_driving_gate_closed)
        .unwrap();
    assert_eq!(pedals(&app, v), (0.0, 0.0, 0.0));
    assert!(!app.world().resource::<MouseDriving>().steering);
}
