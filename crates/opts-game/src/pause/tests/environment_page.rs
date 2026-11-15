//! The environment page of the pause overlay.

use super::*;
use opts_bevy::{CalendarDate, EnvironmentChange, SessionEnvironment};

fn drive_at_noon() -> App {
    let mut app = app();
    app.world_mut().write_message(EnvironmentChange::reset(
        CalendarDate::new(1994, 6, 1),
        12.0 * 3600.0,
        "Clear Summer Day",
    ));
    app.update();
    app
}

fn env(app: &App) -> SessionEnvironment {
    app.world().resource::<SessionEnvironment>().clone()
}

fn open_page(app: &mut App) {
    esc(app);
    click(app, "pause.environment");
    let ids = ids(app);
    assert!(
        ids.contains(&"pause.environment.apply".to_string()),
        "{ids:?}"
    );
    assert!(
        !ids.contains(&"pause.resume".to_string()),
        "one dialog at a time"
    );
}

fn light_row_text(app: &mut App) -> String {
    let kids: Vec<Entity> = app
        .world_mut()
        .query_filtered::<&Children, With<light_row::LightModeButton>>()
        .iter(app.world())
        .flat_map(|c| c.iter().collect::<Vec<_>>())
        .collect();
    kids.iter()
        .filter_map(|k| app.world().get::<Text>(*k).map(|t| t.0.clone()))
        .collect()
}

mod lighting;
mod preview;
mod apply;
