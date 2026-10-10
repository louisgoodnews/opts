//! Automated integration tests validating headless startup, state progression, and clean shutdown.

use bevy::prelude::*;
use opts_game::{
    app_state::AppState, build_app, menu::layout::MainMenuRoot, settings::GameSettings, AppArgs,
    CliArgs,
};
use std::path::PathBuf;

#[test]
fn test_headless_startup_and_state_transition() {
    let args = CliArgs {
        headless: true,
        ..Default::default()
    };

    let mut app = build_app(args);

    // Initial state before stepping must be Loading
    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::Loading
    );

    // Step 5 frames to execute startup systems and allow state transitions
    for _ in 0..5 {
        app.update();
    }

    // State must transition cleanly to MainMenu
    assert_eq!(
        app.world().resource::<State<AppState>>().get(),
        &AppState::MainMenu
    );

    // Verify MainMenuRoot UI hierarchy spawned
    let mut root_query = app
        .world_mut()
        .query_filtered::<Entity, With<MainMenuRoot>>();
    let root_count = root_query.iter(app.world()).count();
    assert_eq!(root_count, 1, "MainMenuRoot entity must be present");

    // Verify AppArgs resource is registered
    let stored_args = app.world().resource::<AppArgs>();
    assert!(stored_args.0.headless);
}

#[test]
fn test_headless_cli_overrides_settings() {
    let test_path = PathBuf::from("/test/maps/Spandau");
    let args = CliArgs {
        headless: true,
        map_path: Some(test_path.clone()),
        ..Default::default()
    };

    let app = build_app(args);

    let settings = app.world().resource::<GameSettings>();
    assert_eq!(settings.recent_map_path, Some(test_path));
}

#[test]
fn test_headless_shutdown_event_handling() {
    let args = CliArgs {
        headless: true,
        ..Default::default()
    };

    let mut app = build_app(args);

    // Step once to settle startup
    app.update();

    // Trigger AppExit event
    app.world_mut().send_event(AppExit::Success);

    // Stepping frame should execute ShutdownPlugin systems cleanly without panics
    app.update();
}
