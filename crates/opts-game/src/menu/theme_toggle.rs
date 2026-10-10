//! Floating theme toggle button and hotkey system for runtime light/dark switching.

use bevy::prelude::*;
use opts_gui::{
    interaction::events::ThemeChanged,
    theme::{color::ColorRole, theme::MaterialTheme},
    tokens::components::{ForegroundRole, RadiusToken},
};

use crate::menu::glass::GlassSurface;
use crate::settings::GameSettings;

/// Marker component for the theme toggle button entity.
#[derive(Component)]
pub struct ThemeToggleBtn;

/// Marker component for the text/glyph inside the theme toggle button.
#[derive(Component)]
pub struct ThemeToggleGlyph;

/// Spawns the floating circular theme toggle button in the top-right corner.
pub fn spawn_theme_toggle(commands: &mut Commands, theme: &MaterialTheme) -> Entity {
    let glyph = if theme.scheme.is_dark() {
        "Light"
    } else {
        "Dark"
    };

    let btn = commands
        .spawn((
            ButtonBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(24.0),
                    right: Val::Px(24.0),
                    min_height: Val::Px(40.0),
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                border_color: BorderColor(theme.scheme.get(ColorRole::Outline)),
                z_index: ZIndex::Local(2),
                ..default()
            },
            ThemeToggleBtn,
            GlassSurface::new(ColorRole::Surface, 0.70),
            RadiusToken(opts_gui::theme::shape::RadiusLevel::Full),
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    glyph,
                    TextStyle {
                        font_size: 14.0,
                        color: theme.scheme.get(ColorRole::OnSurface),
                        ..default()
                    },
                ),
                ThemeToggleGlyph,
                ForegroundRole(ColorRole::OnSurface),
            ));
        })
        .id();

    btn
}

/// System that handles theme toggle clicks or the `F2` hotkey to switch palettes at runtime.
pub fn theme_toggle_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<ThemeToggleBtn>)>,
    theme: Res<MaterialTheme>,
    mut settings: ResMut<GameSettings>,
    mut theme_events: EventWriter<ThemeChanged>,
    mut glyph_query: Query<&mut Text, With<ThemeToggleGlyph>>,
) {
    let mut should_toggle = keyboard.just_pressed(KeyCode::F2);

    for interaction in &interaction_query {
        if *interaction == Interaction::Pressed {
            should_toggle = true;
            break;
        }
    }

    if should_toggle {
        let new_is_dark = !theme.scheme.is_dark();
        tracing::info!(new_is_dark = new_is_dark, "Toggling application theme");

        theme_events.send(ThemeChanged {
            is_dark: new_is_dark,
        });
        settings.is_dark_theme = new_is_dark;
        if let Err(err) = settings.save() {
            tracing::warn!(err = ?err, "Failed to persist theme preference");
        }

        // Update glyph display
        let next_glyph = if new_is_dark { "Light" } else { "Dark" };
        for mut text in &mut glyph_query {
            for section in &mut text.sections {
                section.value = next_glyph.to_string();
            }
        }
    }
}
