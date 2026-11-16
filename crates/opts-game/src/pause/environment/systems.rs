//! Input, refresh, and rebuild-resume systems for the pause menu's *Environment* page.

use super::*;
mod refresh;
pub(crate) use refresh::resume_drive;
pub use refresh::{refresh_environment_page, resume_after_rebuild};

fn close(
    commands: &mut Commands,
    page: &mut EnvironmentPage,
    roots: &Query<Entity, With<EnvironmentOverlay>>,
) {
    for root in roots {
        commands.entity(root).try_despawn();
    }
    page.open = false;
    page.snapshot = None;
}

fn restore(page: &EnvironmentPage, changes: &mut MessageWriter<EnvironmentChange>) {
    if let Some((seconds, weather)) = &page.snapshot {
        changes.write(EnvironmentChange {
            seconds_of_day: Some(*seconds),
            weather: Some(weather.clone()),
            ..default()
        });
    }
}

/// The page's controls, and Esc (cancel).
pub fn environment_page_input(
    mut commands: Commands,
    events: super::params::PageEvents,
    nodes: super::params::PageNodes,
    env: Res<SessionEnvironment>,
    mut page: ResMut<EnvironmentPage>,
    mut changes: MessageWriter<EnvironmentChange>,
) {
    let super::params::PageEvents {
        mut activated,
        mut modal_events,
    } = events;
    let super::params::PageNodes {
        actions,
        roots,
        modal_roots,
    } = nodes;
    for ModalEvent::CancelRequested(modal) in modal_events.read() {
        if roots.contains(*modal) && modal_roots.contains(*modal) {
            restore(&page, &mut changes);
            close(&mut commands, &mut page, &roots);
        }
    }
    for Activated(entity) in activated.read() {
        let Ok((action, weather_id)) = actions.get(*entity) else {
            continue;
        };
        match *action {
            EnvAction::Step(field, delta) if Field::DATE.contains(&field) => {
                calendar::step_date(&mut page.draft_date, field, delta);
            }
            EnvAction::Step(field, delta) => {
                let (mut h, mut m, _) = env.hms();
                calendar::step_time(&mut h, &mut m, field, delta, false);
                changes.write(EnvironmentChange {
                    seconds_of_day: Some(f64::from(h * 3600 + m * 60)),
                    ..default()
                });
            }
            EnvAction::Weather => {
                if let Some(WeatherId(id)) = weather_id {
                    changes.write(EnvironmentChange {
                        weather: Some(id.clone()),
                        ..default()
                    });
                }
            }
            EnvAction::LightMode => {}
            EnvAction::Apply => {
                changes.write(EnvironmentChange {
                    date: Some(page.draft_date),
                    map_date: MapDatePolicy::Follow,
                    ..default()
                });
                close(&mut commands, &mut page, &roots);
            }
            EnvAction::Cancel => {
                restore(&page, &mut changes);
                close(&mut commands, &mut page, &roots);
            }
        }
    }
}

/// Registers the page.
pub fn build(app: &mut App) {
    app.init_resource::<EnvironmentPage>();
}
