//! The developer panel: its text entity and the F10 / Shift+F10 toggles.

use bevy::prelude::*;

use crate::in_game::types::InGameEntity;

/// Marker for the toggleable diagnostic overlay text.
#[derive(Component)]
pub struct DebugOverlayText;

/// Spawns the developer panel (hidden unless *Settings → Advanced → Developer tools* is on). It
/// uses the instrument tokens like the rest of the HUD.
pub fn spawn_debug_overlay(commands: &mut Commands) {
    commands.spawn((
        (
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor::default(),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(96.0),
                left: Val::Px(20.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderColor::all(Color::NONE),
            Visibility::Hidden,
        ),
        crate::in_game::instruments::InstrumentPanel,
        crate::in_game::instruments::InstrumentInk::Text,
        DebugOverlayText,
        InGameEntity,
    ));
}

/// F10 switches the developer panel by changing its setting (so the page and the key agree and
/// the choice is remembered).
pub fn toggle_developer_panel(
    keyboard: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<opts_bevy::ActionKeys>>,
    mut api: crate::settings_registry::SettingsApi,
) {
    if !opts_bevy::action_keys_of(&action_keys).just_pressed(&keyboard, "developer_panel") {
        return;
    }
    let on = api.text("advanced.developer_panel") == "true";
    let next = if on { "false" } else { "true" };
    if let Err(e) = api.change(
        "advanced.developer_panel",
        Some(next),
        crate::settings_registry::ChangeSource::Page,
        true,
    ) {
        tracing::warn!("developer panel setting: {e}");
    }
}

/// Shift + F10 switches the developer panel between its short and its detailed page.
pub fn toggle_developer_detail(
    keyboard: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<opts_bevy::ActionKeys>>,
    mut detail: ResMut<super::super::overlay_info::DeveloperDetail>,
) {
    if opts_bevy::action_keys_of(&action_keys).just_pressed(&keyboard, "developer_panel_detail") {
        detail.full = !detail.full;
    }
}
