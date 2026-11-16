use super::*;

pub(super) fn escape(app: &mut App) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: Key::Escape,
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        for _ in 0..3 {
            app.update();
        }
    }
}

/// Switches the language of a running app (`en`, `de`, `en-XA`).
pub fn set_locale(app: &mut App, locale: &str) {
    app.world_mut()
        .resource_mut::<Localization>()
        .set_locale(locale);
    for _ in 0..4 {
        app.update();
    }
}

/// Switches the theme of a running app.
pub fn set_theme(app: &mut App, kind: ThemeKind, accent: AccentPreset) {
    app.insert_resource(preferences(kind, accent));
    for _ in 0..4 {
        app.update();
    }
}
