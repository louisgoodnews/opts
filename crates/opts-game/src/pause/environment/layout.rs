//! Widgets of the Environment page: labelled text, the stepper for a date or time field, the weather chips, the buttons and the action row.

use super::*;

pub(super) fn text(key: &'static str, scale: TypeScale, role: ColorRole) -> impl Bundle {
    (
        Text::new(key),
        L10nKey(key),
        TextColor::default(),
        TypeStyleToken(scale),
        ForegroundRole(role),
    )
}

pub(super) fn stepper(commands: &mut Commands, field: Field) -> Entity {
    calendar::spawn_stepper(
        commands,
        field,
        String::new(),
        EnvValue(field),
        "pause.environment",
        move |d| EnvAction::Step(field, d),
    )
}

pub(super) fn weather_chips(
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

pub(super) fn button(
    commands: &mut Commands,
    key: &'static str,
    filled: bool,
    action: EnvAction,
) -> Entity {
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
pub(super) fn section_title(commands: &mut Commands, key: &'static str) -> Entity {
    commands
        .spawn(text(key, TypeScale::TitleMedium, ColorRole::OnSurface))
        .id()
}

/// The *Cancel* and *Apply* buttons.
pub(super) fn action_row(commands: &mut Commands) -> Entity {
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
