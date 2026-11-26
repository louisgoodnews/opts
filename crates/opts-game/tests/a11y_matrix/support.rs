//! The screen list of the accessibility matrix, and the language and theme switches of a running
//! screen app (used by the contrast and the control checks only).

use bevy::prelude::*;
use opts_gui::l10n::Localization;
use opts_gui::theme::mode::ThemeKind;
use opts_gui::theme::AccentPreset;

use crate::layout::preferences;
use crate::screens::Screen;

impl Screen {
    pub const ALL: [Screen; 8] = [
        Screen::Home,
        Screen::Help,
        Screen::Settings,
        Screen::Wizard,
        Screen::Pause,
        Screen::FirstRunChoose,
        Screen::Welcome,
        Screen::Summary,
    ];
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
