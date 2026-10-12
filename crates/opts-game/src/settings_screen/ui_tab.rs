//! UI & Themes settings tab: theme mode, auto-switch times, accent, scale, density, motion.
//!
//! Every choice is a [`ThemeChoiceButton`]; `theme_choice.rs` applies it.

use bevy::prelude::*;
use opts_gui::components::Button;
use opts_gui::prelude::*;

use crate::menu::glass::GlassSurface;
use crate::settings::GameSettings;
use crate::settings_screen::display_tab::choice_colors;
use crate::settings_screen::types::{
    AutoTimeLabel, ThemeChoice, ThemeChoiceButton, ThemeFileButton,
};
use opts_gui::theme::mode::format_time;

fn heading(parent: &mut ChildSpawnerCommands, theme: &MaterialTheme, text: &str) {
    parent.spawn((
        (
            Text::new(text),
            TextColor(theme.scheme.get(ColorRole::OnSurface)),
        ),
        TypeStyleToken(TypeScale::TitleMedium),
        ForegroundRole(ColorRole::OnSurface),
    ));
}

fn hint(parent: &mut ChildSpawnerCommands, theme: &MaterialTheme, text: &str) {
    parent.spawn((
        (
            Text::new(text),
            TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
        ),
        TypeStyleToken(TypeScale::Label),
        ForegroundRole(ColorRole::OnSurfaceVariant),
    ));
}

fn choice_button(
    row: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    settings: &GameSettings,
    choice: ThemeChoice,
) {
    let selected = choice.is_selected(settings);
    let (border, bg) = choice_colors(theme, selected);
    row.spawn((
        (
            Button,
            Node {
                padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                border: UiRect::all(Val::Px(if selected { 2.0 } else { 1.0 })),
                ..default()
            },
            BorderColor::all(border),
            BackgroundColor(bg),
        ),
        M3Button::default(),
        ThemeChoiceButton(choice),
        GlassSurface::new(
            if selected {
                ColorRole::PrimaryContainer
            } else {
                ColorRole::SurfaceContainerHigh
            },
            0.60,
        ),
        RadiusToken(RadiusLevel::Small),
        Focusable { tab_index: 0 },
    ))
    .with_children(|btn| {
        btn.spawn((
            Text::new(choice.label()),
            TextColor(theme.scheme.get(if selected {
                ColorRole::OnPrimaryContainer
            } else {
                ColorRole::OnSurface
            })),
            TypeStyleToken(TypeScale::BodyMedium),
        ));
    });
}

fn choice_row(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    settings: &GameSettings,
    choices: &[ThemeChoice],
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            row_gap: Val::Px(8.0),
            flex_wrap: FlexWrap::Wrap,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            for &c in choices {
                choice_button(row, theme, settings, c);
            }
        });
}

fn time_row(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    settings: &GameSettings,
    title: &str,
    light: bool,
) {
    let (back, forward, time) = if light {
        (
            ThemeChoice::LightFromStep(-super::theme_choice::TIME_STEP_MINUTES),
            ThemeChoice::LightFromStep(super::theme_choice::TIME_STEP_MINUTES),
            settings.light_from(),
        )
    } else {
        (
            ThemeChoice::DarkFromStep(-super::theme_choice::TIME_STEP_MINUTES),
            ThemeChoice::DarkFromStep(super::theme_choice::TIME_STEP_MINUTES),
            settings.dark_from(),
        )
    };
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new(title),
                TextColor(theme.scheme.get(ColorRole::OnSurface)),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(ColorRole::OnSurface),
                Node {
                    min_width: Val::Px(110.0),
                    ..default()
                },
            ));
            choice_button(row, theme, settings, back);
            row.spawn((
                Text::new(format_time(time)),
                TextColor(theme.scheme.get(ColorRole::OnSurface)),
                TypeStyleToken(TypeScale::TitleLarge),
                ForegroundRole(ColorRole::OnSurface),
                AutoTimeLabel(light),
            ));
            choice_button(row, theme, settings, forward);
        });
}

fn theme_file_row(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    settings: &GameSettings,
) {
    let mut options: Vec<Option<String>> = vec![None];
    options.extend(
        crate::theme_files::discover_theme_files()
            .into_iter()
            .map(Some),
    );
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            row_gap: Val::Px(8.0),
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .with_children(|row| {
            for option in options {
                let selected = option.clone().unwrap_or_default() == settings.theme_file;
                let (border, bg) = choice_colors(theme, selected);
                let label = option.clone().unwrap_or_else(|| "Built-in".to_string());
                row.spawn((
                    (
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                            border: UiRect::all(Val::Px(if selected { 2.0 } else { 1.0 })),
                            ..default()
                        },
                        BorderColor::all(border),
                        BackgroundColor(bg),
                    ),
                    M3Button::default(),
                    ThemeFileButton(option),
                    RadiusToken(RadiusLevel::Small),
                    Focusable { tab_index: 0 },
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(label),
                        TextColor(theme.scheme.get(ColorRole::OnSurface)),
                        TypeStyleToken(TypeScale::BodyMedium),
                    ));
                });
            }
        });
}

/// Spawns the UI & Themes tab content in two columns (themes on the left, interface on the right).
pub fn spawn_ui_tab(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    settings: &GameSettings,
) {
    let column = || Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(8.0),
        flex_basis: Val::Px(420.0),
        flex_grow: 1.0,
        flex_shrink: 1.0,
        max_width: Val::Px(560.0),
        ..default()
    };
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(32.0),
            row_gap: Val::Px(8.0),
            align_items: AlignItems::FlexStart,
            ..default()
        })
        .with_children(|page| {
            page.spawn(column()).with_children(|col| {
                heading(col, theme, "Theme");
                choice_row(col, theme, settings, &ThemeChoice::modes());
                hint(
                    col,
                    theme,
                    "System follows your desktop; Auto (clock) switches by the time of day on this device.",
                );
                time_row(col, theme, settings, "Light from", true);
                time_row(col, theme, settings, "Dark from", false);

                heading(col, theme, "High contrast");
                choice_row(
                    col,
                    theme,
                    settings,
                    &[ThemeChoice::HighContrast(false), ThemeChoice::HighContrast(true)],
                );
                hint(col, theme, "Maximum contrast; never switched on automatically.");

                heading(col, theme, "Accent colour");
                choice_row(col, theme, settings, &ThemeChoice::accents());
            });
            page.spawn(column()).with_children(|col| {
                heading(col, theme, "Interface size");
                choice_row(col, theme, settings, &ThemeChoice::scales());
                heading(col, theme, "Density");
                choice_row(col, theme, settings, &ThemeChoice::densities());

                heading(col, theme, "Reduce motion");
                choice_row(
                    col,
                    theme,
                    settings,
                    &[ThemeChoice::ReducedMotion(false), ThemeChoice::ReducedMotion(true)],
                );

                heading(col, theme, "Theme file");
                theme_file_row(col, theme, settings);
                hint(
                    col,
                    theme,
                    "RON files in the themes folder of the configuration directory; colours that fail the contrast rules are corrected.",
                );
            });
        });
}
