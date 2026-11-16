//! The wheel press that switches the mouse driving mode on and off.

use super::*;

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
