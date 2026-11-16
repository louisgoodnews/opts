//! Spawns the Environment page: the overlay, its title, the fields, the weather list and the action row.

use super::layout::*;
use super::*;

pub(super) fn spawn_page(
    commands: &mut Commands,
    weather_list: &WeatherCatalogue,
    loc: &Localization,
) {
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
    let light = crate::pause::light_row::spawn(commands);
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
