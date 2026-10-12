//! Clicks on the rendering choices of the Display tab (texture filtering, colour look).

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::look::ActiveLook;
use crate::settings::GameSettings;
use crate::settings_screen::display_tab::choice_colors;
use crate::settings_screen::types::{RenderChoice, RenderChoiceButton};

/// Stores the clicked choice in the settings, applies it live and persists it.
pub fn handle_render_choice_clicks(
    mut events: MessageReader<ButtonClicked>,
    clicked: Query<&RenderChoiceButton>,
    mut buttons: Query<(&RenderChoiceButton, &mut BorderColor, &mut BackgroundColor)>,
    mut settings: ResMut<GameSettings>,
    mut active: ResMut<ActiveLook>,
    theme: Res<MaterialTheme>,
) {
    let Some(choice) = events
        .read()
        .filter_map(|e| clicked.get(e.0).ok().map(|b| b.0))
        .last()
    else {
        return;
    };
    apply_choice(&mut settings, &mut active, choice);
    for (button, mut border, mut bg) in &mut buttons {
        if button.0.same_group(choice) {
            let (b, g) = choice_colors(&theme, button.0 == choice);
            *border = BorderColor::all(b);
            *bg = BackgroundColor(g);
        }
    }
    if let Err(err) = settings.save() {
        tracing::warn!(err = ?err, "Failed to persist the rendering choice to SQLite");
    }
}

/// Stores `choice` in `settings` and makes it take effect.
pub fn apply_choice(settings: &mut GameSettings, active: &mut ActiveLook, choice: RenderChoice) {
    match choice {
        RenderChoice::Filtering(f) => {
            settings.texture_filtering = f;
            opts_bevy::set_texture_filtering(f);
        }
        RenderChoice::Look(l) => {
            settings.look = l;
            active.0 = l;
        }
    }
    tracing::info!(?choice, "rendering choice changed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::look::Look;
    use opts_bevy::TextureFiltering;

    #[test]
    fn choices_update_settings_and_take_effect_live() {
        let mut settings = GameSettings::default();
        let mut active = ActiveLook::default();
        assert_eq!(active.0, Look::Vivid);
        apply_choice(
            &mut settings,
            &mut active,
            RenderChoice::Look(Look::Natural),
        );
        assert_eq!((settings.look, active.0), (Look::Natural, Look::Natural));
        apply_choice(
            &mut settings,
            &mut active,
            RenderChoice::Filtering(TextureFiltering::X4),
        );
        assert_eq!(settings.texture_filtering, TextureFiltering::X4);
        assert_eq!(opts_bevy::texture_filtering(), TextureFiltering::X4);
        opts_bevy::set_texture_filtering(TextureFiltering::default());
    }

    #[test]
    fn choices_know_their_group_and_selection() {
        let settings = GameSettings::default();
        let a = RenderChoice::Look(Look::Vivid);
        assert!(a.same_group(RenderChoice::Look(Look::Natural)));
        assert!(!a.same_group(RenderChoice::Filtering(TextureFiltering::X8)));
        assert!(a.is_selected(&settings));
        assert!(RenderChoice::Filtering(TextureFiltering::X16).is_selected(&settings));
        assert!(!RenderChoice::Filtering(TextureFiltering::Off).is_selected(&settings));
    }
}
