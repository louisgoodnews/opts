//! Input, refresh, and rebuild-resume systems for the pause menu's *Environment* page.

use super::*;

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

/// Keeps the values, the weather chips and the rebuild line current.
pub fn refresh_environment_page(
    facts: super::params::PageFacts,
    mut values: Query<(&EnvValue, &mut Text)>,
    chips: Query<(
        Entity,
        &WeatherId,
        &ChoiceMark,
        Has<opts_gui::interaction::state::Selected>,
    )>,
    mut lines: Query<(&mut Node, &mut L10nArgs), With<RebuildLine>>,
    mut nodes: Query<&mut Node, Without<RebuildLine>>,
    mut commands: Commands,
) {
    let super::params::PageFacts {
        page,
        env,
        rebuild,
        loc,
        weather,
    } = facts;
    if !page.open {
        return;
    }
    let (h, m, _) = env.hms();
    for (v, mut t) in &mut values {
        let new = if Field::DATE.contains(&v.0) {
            calendar::date_value(page.draft_date, v.0, |mo| loc.t(calendar::month_key(mo)))
        } else {
            calendar::time_value(h, m, v.0)
        };
        if t.0 != new {
            t.0 = new;
        }
    }
    let current = weather.canonical_id(env.weather());
    for (entity, WeatherId(id), mark, was) in &chips {
        let on = *id == current;
        if on != was {
            calendar::show_chosen(&mut commands, &mut nodes, entity, mark, on);
        }
    }
    for (mut node, mut args) in &mut lines {
        let display = if rebuild.is_pending() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        let want = L10nArgs(vec![
            ("pending".into(), (rebuild.pending_tiles as usize).into()),
            ("total".into(), (rebuild.total_tiles as usize).into()),
        ]);
        if *args != want {
            *args = want;
        }
    }
}

/// Closes the overlay and resumes; while the map is rebuilt for a new date the simulation
/// stays paused until [`resume_after_rebuild`] sees the rebuild finish.
pub(crate) fn resume_drive(
    commands: &mut Commands,
    pause: &mut super::super::Pause,
    time: &mut Time<Virtual>,
    rebuild: Option<&opts_bevy::ChronoRebuild>,
) {
    let waiting = rebuild.is_some_and(opts_bevy::ChronoRebuild::is_pending);
    super::super::close_pause(commands, pause, time, !waiting);
    pause.resume_after_rebuild = waiting && pause.resume_time;
}

/// Resumes a drive whose *Resume* waited for a map rebuild.
pub fn resume_after_rebuild(
    rebuild: Option<Res<opts_bevy::ChronoRebuild>>,
    mut pause: ResMut<super::super::Pause>,
    mut time: ResMut<Time<Virtual>>,
) {
    if !pause.resume_after_rebuild || pause.open {
        return;
    }
    if !rebuild.is_some_and(|r| r.is_pending()) {
        pause.resume_after_rebuild = false;
        time.unpause();
        tracing::info!("map rebuilt for the new date: drive resumed");
    }
}

/// Registers the page.
pub fn build(app: &mut App) {
    app.init_resource::<EnvironmentPage>();
}
