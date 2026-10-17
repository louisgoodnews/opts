//! The pause menu's *Environment* page: date, time of day and weather of the running drive.
//!
//! Time and weather change at once while the page is open (the sky and the light follow behind
//! the overlay); the date is a draft that applies with *Apply* (it may change the map's content,
//! Chrono). *Cancel* and Esc restore the time and weather the page opened with. All changes go
//! through [`EnvironmentChange`] (`docs/src/architecture/session-environment.md`).

use bevy::prelude::*;
use opts_bevy::{
    CalendarDate, ChronoRebuild, EnvironmentChange, MapDatePolicy, SessionEnvironment,
};
use opts_gui::components::modal::{ModalBuilder, ModalEvent, ModalRoot};
use opts_gui::interaction::activation::Activated;
use opts_gui::l10n::{L10nArgs, L10nKey, Localization};
use opts_gui::prelude::*;

use crate::new_game_screen::weather::{preset_of, PRESETS};
use crate::shell::calendar::{self, ChoiceMark, Field};
use crate::shell::keys;

/// Marks the page's modal root.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct EnvironmentOverlay;

/// What a control of the page does.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvAction {
    /// Change a date (draft) or time (live) field by `delta`.
    Step(Field, i32),
    /// Choose a weather preset (live).
    Weather(&'static str),
    /// Apply the date and close.
    Apply,
    /// Restore time and weather and close.
    Cancel,
}

/// A value text of a stepper.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvValue(pub Field);

/// The line that reports a running map rebuild.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct RebuildLine;

/// State of the page.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct EnvironmentPage {
    /// The page is open.
    pub open: bool,
    /// Time of day and weather when the page opened (restored by *Cancel*).
    pub snapshot: Option<(f64, String)>,
    /// The date being chosen (applied by *Apply*).
    pub draft_date: CalendarDate,
}

/// Opens the page (the caller has put the pause overlay aside).
pub fn open_environment_page(commands: &mut Commands) {
    commands.queue(|world: &mut World| {
        if world
            .query_filtered::<(), With<EnvironmentOverlay>>()
            .iter(world)
            .next()
            .is_some()
        {
            return;
        }
        let env = world
            .get_resource::<SessionEnvironment>()
            .cloned()
            .unwrap_or_default();
        *world.resource_mut::<EnvironmentPage>() = EnvironmentPage {
            open: true,
            snapshot: Some((env.seconds_of_day(), env.weather().to_owned())),
            draft_date: env.date(),
        };
        let mut commands = world.commands();
        spawn_page(&mut commands);
        world.flush();
    });
}

fn text(key: &'static str, scale: TypeScale, role: ColorRole) -> impl Bundle {
    (
        Text::new(key),
        L10nKey(key),
        TextColor::default(),
        TypeStyleToken(scale),
        ForegroundRole(role),
    )
}

fn stepper(commands: &mut Commands, field: Field) -> Entity {
    calendar::spawn_stepper(
        commands,
        field,
        String::new(),
        EnvValue(field),
        "pause.environment",
        move |d| EnvAction::Step(field, d),
    )
}

fn weather_chips(commands: &mut Commands) -> Entity {
    let wrap = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(8.0),
            ..default()
        })
        .id();
    for p in PRESETS {
        let chip = calendar::spawn_choice_frame(
            commands,
            format!("pause.environment.weather.{}", opts_gui::ui_id::slug(p.id)),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                min_height: Val::Px(44.0),
                ..default()
            },
        );
        let name = commands
            .spawn(text(
                p.name_key,
                TypeScale::BodyMedium,
                ColorRole::OnSurface,
            ))
            .id();
        commands
            .entity(chip)
            .insert(EnvAction::Weather(p.id))
            .add_child(name);
        commands.entity(wrap).add_child(chip);
    }
    wrap
}

fn button(commands: &mut Commands, key: &'static str, filled: bool, action: EnvAction) -> Entity {
    let builder = if filled {
        ButtonBuilder::filled(key)
    } else {
        ButtonBuilder::text(key)
    };
    let b = builder.with_key(key).spawn(commands);
    let id = if action == EnvAction::Apply {
        "pause.environment.apply"
    } else {
        "pause.environment.cancel"
    };
    commands.entity(b).insert((action, UiId::new(id)));
    b
}

fn spawn_page(commands: &mut Commands) {
    let (root, card) = ModalBuilder::new()
        .with_size_percent(60.0, 80.0)
        .with_width_range(420.0, 720.0)
        .spawn(commands);
    commands
        .entity(root)
        .insert((EnvironmentOverlay, Name::new("EnvironmentOverlay")));
    let title = commands
        .spawn(text(
            keys::ENV_TITLE,
            TypeScale::TitleLarge,
            ColorRole::OnSurface,
        ))
        .id();
    let note = commands
        .spawn(text(
            keys::ENV_NOTE,
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let date_title = commands
        .spawn(text(
            keys::ENV_DATE,
            TypeScale::TitleMedium,
            ColorRole::OnSurface,
        ))
        .id();
    let date_steppers: Vec<Entity> = Field::DATE.iter().map(|f| stepper(commands, *f)).collect();
    let date = calendar::spawn_stepper_row(commands, &date_steppers);
    let time_title = commands
        .spawn(text(
            keys::ENV_TIME,
            TypeScale::TitleMedium,
            ColorRole::OnSurface,
        ))
        .id();
    let time_steppers: Vec<Entity> = Field::TIME.iter().map(|f| stepper(commands, *f)).collect();
    let time = calendar::spawn_stepper_row(commands, &time_steppers);
    let weather_title = commands
        .spawn(text(
            keys::ENV_WEATHER,
            TypeScale::TitleMedium,
            ColorRole::OnSurface,
        ))
        .id();
    let weather = weather_chips(commands);
    let rebuild = commands
        .spawn((
            text(keys::ENV_REBUILDING, TypeScale::Label, ColorRole::Info),
            L10nArgs(vec![
                ("pending".into(), 0.into()),
                ("total".into(), 0.into()),
            ]),
            RebuildLine,
            Node {
                display: Display::None,
                ..default()
            },
        ))
        .id();
    let buttons = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .id();
    let cancel = button(commands, keys::CANCEL, false, EnvAction::Cancel);
    let apply = button(commands, keys::ENV_APPLY, true, EnvAction::Apply);
    commands.entity(apply).insert(DefaultFocus);
    commands.entity(buttons).add_children(&[cancel, apply]);
    let body = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            overflow: Overflow::scroll_y(),
            width: Val::Percent(100.0),
            ..default()
        })
        .id();
    commands.entity(body).add_children(&[
        title,
        note,
        date_title,
        date,
        time_title,
        time,
        weather_title,
        weather,
        rebuild,
        buttons,
    ]);
    commands.entity(card).add_child(body);
}

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
#[allow(clippy::too_many_arguments)]
pub fn environment_page_input(
    mut commands: Commands,
    mut activated: MessageReader<Activated>,
    mut modal_events: MessageReader<ModalEvent>,
    actions: Query<&EnvAction>,
    roots: Query<Entity, With<EnvironmentOverlay>>,
    modal_roots: Query<(), With<ModalRoot>>,
    env: Res<SessionEnvironment>,
    mut page: ResMut<EnvironmentPage>,
    mut changes: MessageWriter<EnvironmentChange>,
) {
    for ModalEvent::CancelRequested(modal) in modal_events.read() {
        if roots.contains(*modal) && modal_roots.contains(*modal) {
            restore(&page, &mut changes);
            close(&mut commands, &mut page, &roots);
        }
    }
    for Activated(entity) in activated.read() {
        let Ok(action) = actions.get(*entity) else {
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
            EnvAction::Weather(id) => {
                changes.write(EnvironmentChange {
                    weather: Some(id.to_owned()),
                    ..default()
                });
            }
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
#[allow(clippy::too_many_arguments)]
pub fn refresh_environment_page(
    page: Res<EnvironmentPage>,
    env: Res<SessionEnvironment>,
    rebuild: Res<ChronoRebuild>,
    loc: Res<Localization>,
    mut values: Query<(&EnvValue, &mut Text)>,
    chips: Query<(
        Entity,
        &EnvAction,
        &ChoiceMark,
        Has<opts_gui::interaction::state::Selected>,
    )>,
    mut lines: Query<(&mut Node, &mut L10nArgs), With<RebuildLine>>,
    mut nodes: Query<&mut Node, Without<RebuildLine>>,
    mut commands: Commands,
) {
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
    let current = preset_of(env.weather()).id;
    for (entity, action, mark, was) in &chips {
        if let EnvAction::Weather(id) = action {
            let on = *id == current;
            if on != was {
                calendar::show_chosen(&mut commands, &mut nodes, entity, mark, on);
            }
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
    pause: &mut super::Pause,
    time: &mut Time<Virtual>,
    rebuild: Option<&opts_bevy::ChronoRebuild>,
) {
    let waiting = rebuild.is_some_and(opts_bevy::ChronoRebuild::is_pending);
    super::close_pause(commands, pause, time, !waiting);
    pause.resume_after_rebuild = waiting && pause.resume_time;
}

/// Resumes a drive whose *Resume* waited for a map rebuild.
pub fn resume_after_rebuild(
    rebuild: Option<Res<opts_bevy::ChronoRebuild>>,
    mut pause: ResMut<super::Pause>,
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
