//! Floating theme toggle button and hotkey system for runtime light/dark switching.
//!
//! The toggle flips between the fixed modes Dark and Light (it is the quick switch; System and
//! Auto (clock) are chosen in the settings). Choosing it leaves the high-contrast switch alone.

use bevy::prelude::*;
use opts_gui::components::Button;
use opts_gui::l10n::{keys::menu as keys, L10nKey};
use opts_gui::theme::{ThemeKind, ThemeMode};
use opts_gui::{
    interaction::activation::Activated,
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

/// The message key of the toggle: it offers the mode a click switches **to**.
fn theme_glyph_key(kind: ThemeKind) -> &'static str {
    if kind.is_dark() {
        keys::THEME_LIGHT
    } else {
        keys::THEME_DARK
    }
}

/// Spawns the floating circular theme toggle button in the top-right corner.
pub fn spawn_theme_toggle(commands: &mut Commands, theme: &MaterialTheme) -> Entity {
    let glyph = theme_glyph_key(theme.kind());

    let btn = commands
        .spawn((
            (
                Button,
                Node {
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
                BorderColor::all(theme.scheme.get(ColorRole::Outline)),
                ZIndex(2),
            ),
            ThemeToggleBtn,
            GlassSurface::new(ColorRole::Surface, 0.70),
            RadiusToken(opts_gui::theme::shape::RadiusLevel::Full),
        ))
        .with_children(|parent| {
            parent.spawn((
                (
                    Text::new(glyph),
                    L10nKey(glyph),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnSurface)),
                ),
                ThemeToggleGlyph,
                ForegroundRole(ColorRole::OnSurface),
            ));
        })
        .id();

    btn
}

/// System that handles theme toggle clicks or the `F2` hotkey to switch Dark and Light at runtime.
pub fn theme_toggle_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut activated: MessageReader<Activated>,
    toggle_buttons: Query<(), With<ThemeToggleBtn>>,
    theme: Res<MaterialTheme>,
    mut settings: ResMut<GameSettings>,
) {
    let mut should_toggle = keyboard.just_pressed(KeyCode::F2);

    for Activated(entity) in activated.read() {
        if toggle_buttons.contains(*entity) {
            should_toggle = true;
        }
    }

    if should_toggle {
        let mode = if theme.kind().is_dark() {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        };
        tracing::info!(?mode, "Toggling application theme");
        settings.theme_mode = mode;
        settings.high_contrast = false;
        if let Err(err) = settings.save() {
            tracing::warn!(err = ?err, "Failed to persist theme preference");
        }
    }
}

/// Keeps the toggle's label in step with the active theme (whoever changed it).
pub fn sync_theme_toggle_glyph(
    theme: Res<MaterialTheme>,
    mut glyphs: Query<&mut L10nKey, With<ThemeToggleGlyph>>,
) {
    if !theme.is_changed() {
        return;
    }
    let next = theme_glyph_key(theme.kind());
    for mut key in &mut glyphs {
        if key.0 != next {
            key.0 = next;
        }
    }
}
