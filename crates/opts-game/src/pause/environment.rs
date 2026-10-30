//! The pause menu's *Environment* page: date, time of day and weather of the running drive.
//!
//! Time and weather change at once while the page is open (the sky and the light follow behind
//! the overlay); the date is a draft that applies with *Apply* (it may change the map's content,
//! Chrono). *Cancel* and Esc restore the time and weather the page opened with. All changes go
//! through [`EnvironmentChange`] (`docs/src/architecture/session-environment.md`).

use bevy::prelude::*;
use opts_bevy::{
    CalendarDate, ChronoRebuild, EnvironmentChange, MapDatePolicy, SessionEnvironment,
    WeatherCatalogue,
};
use opts_gui::components::modal::{ModalBuilder, ModalEvent, ModalRoot};
use opts_gui::interaction::activation::Activated;
use opts_gui::l10n::{L10nArgs, L10nKey, Localization};
use opts_gui::prelude::*;

use crate::new_game_screen::weather::{presets, Label};
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
    /// Choose a weather preset (live); the id is the control's [`WeatherId`].
    Weather,
    /// Switch the lighting quality between automatic and custom (`graphics.light_quality_mode`).
    LightMode,
    /// Apply the date and close.
    Apply,
    /// Restore time and weather and close.
    Cancel,
}

/// The preset id of a weather chip.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct WeatherId(pub String);

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
        let weather = world
            .get_resource::<WeatherCatalogue>()
            .cloned()
            .unwrap_or_default();
        let loc = world
            .get_resource::<Localization>()
            .cloned()
            .unwrap_or_else(|| Localization::new("en"));
        let mut commands = world.commands();
        spawn_page(&mut commands, &weather, &loc);
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

fn weather_chips(
    commands: &mut Commands,
    weather: &WeatherCatalogue,
    loc: &Localization,
) -> Entity {
    let wrap = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            row_gap: Val::Px(8.0),
            ..default()
        })
        .id();
    for p in presets(weather, loc) {
        let chip = calendar::spawn_choice_frame(
            commands,
            format!("pause.environment.weather.{}", opts_gui::ui_id::slug(&p.id)),
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
                min_height: Val::Px(44.0),
                ..default()
            },
        );
        let name = match p.name {
            Label::Key(key) => commands
                .spawn(text(key, TypeScale::BodyMedium, ColorRole::OnSurface))
                .id(),
            Label::Text(t) => commands
                .spawn((
                    Text::new(t),
                    TextColor::default(),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(ColorRole::OnSurface),
                ))
                .id(),
        };
        commands
            .entity(chip)
            .insert((EnvAction::Weather, WeatherId(p.id)))
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

/// A section heading of the page.
fn section_title(commands: &mut Commands, key: &'static str) -> Entity {
    commands
        .spawn(text(key, TypeScale::TitleMedium, ColorRole::OnSurface))
        .id()
}

/// The *Cancel* and *Apply* buttons.
fn action_row(commands: &mut Commands) -> Entity {
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
    buttons
}

fn spawn_page(commands: &mut Commands, weather_list: &WeatherCatalogue, loc: &Localization) {
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
    let date_title = section_title(commands, keys::ENV_DATE);
    let date_steppers: Vec<Entity> = Field::DATE.iter().map(|f| stepper(commands, *f)).collect();
    let date = calendar::spawn_stepper_row(commands, &date_steppers);
    let time_title = section_title(commands, keys::ENV_TIME);
    let time_steppers: Vec<Entity> = Field::TIME.iter().map(|f| stepper(commands, *f)).collect();
    let time = calendar::spawn_stepper_row(commands, &time_steppers);
    let weather_title = section_title(commands, keys::ENV_WEATHER);
    let weather = weather_chips(commands, weather_list, loc);
    let light_title = section_title(commands, "env-light-quality");
    let light = super::light_row::spawn(commands);
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
    let buttons = action_row(commands);
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
        light_title,
        light,
        rebuild,
        buttons,
    ]);
    commands.entity(card).add_child(body);
}

mod systems;
pub(crate) use systems::resume_drive;
pub use systems::{build, environment_page_input, refresh_environment_page, resume_after_rebuild};
