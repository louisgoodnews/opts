//! The *Lighting quality* row of the Environment page: one button that names the mode (and, in
//! automatic mode, the ladder level) and toggles `graphics.light_quality_mode`
//! (`docs/src/architecture/lighting-system.md` section 9.1).

use bevy::prelude::*;
use opts_gui::interaction::activation::Activated;
use opts_gui::l10n::Localization;
use opts_gui::prelude::*;

use super::environment::{EnvAction, EnvironmentPage};

/// The button of the lighting quality row; its text names the mode (and the level).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct LightModeButton;

/// Shown for the frame before `refresh_light_row` names the mode.
const PLACEHOLDER: &str = "\u{2026}";

/// Spawns the row (a button inside a row container) and returns the container.
pub fn spawn(commands: &mut Commands) -> Entity {
    // The text is set by `refresh_light_row` from the messages; this is a placeholder glyph.
    let light_button = ButtonBuilder::outlined(PLACEHOLDER).spawn(commands);
    commands.entity(light_button).insert((
        EnvAction::LightMode,
        LightModeButton,
        UiId::new("pause.environment.light-quality"),
    ));
    // The button keeps its own width instead of filling the card.
    commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            ..default()
        })
        .add_child(light_button)
        .id()
}

/// The lighting quality row: toggles `graphics.light_quality_mode` between automatic and custom.
pub fn environment_light_toggle(
    mut commands: Commands,
    mut activated: MessageReader<Activated>,
    actions: Query<&EnvAction>,
) {
    for Activated(entity) in activated.read() {
        if actions.get(*entity) != Ok(&EnvAction::LightMode) {
            continue;
        }
        commands.queue(|world: &mut World| {
            use crate::settings_registry::lighting::MODE_KEY;
            if !world.contains_resource::<crate::settings_registry::SettingsRegistry>() {
                return;
            }
            let next = if crate::settings_registry::lighting::mode_of(
                world.resource::<crate::settings::GameSettings>(),
            ) == opts_bevy::light_quality::LightMode::Custom
            {
                "automatic"
            } else {
                "custom"
            };
            let _ = crate::settings_registry::apply_change(
                world,
                MODE_KEY,
                Some(next),
                crate::settings_registry::ChangeSource::Page,
            );
        });
    }
}

/// Names the mode (and, in automatic mode, the level) on the lighting quality button.
pub fn refresh_light_row(
    page: Res<EnvironmentPage>,
    loc: Res<Localization>,
    quality: Option<Res<opts_bevy::light_quality::LightQuality>>,
    buttons: Query<&Children, With<LightModeButton>>,
    mut texts: Query<&mut Text>,
) {
    if !page.open {
        return;
    }
    let q = quality.map_or_else(opts_bevy::light_quality::LightQuality::default, |q| *q);
    let want = match q.mode {
        opts_bevy::light_quality::LightMode::Custom => loc.t("env-light-custom"),
        opts_bevy::light_quality::LightMode::Automatic => loc.t_args(
            "env-light-automatic",
            &[
                ("level".into(), usize::from(q.level).into()),
                (
                    "name".into(),
                    loc.t(crate::settings_screen::page::level_key(q.level))
                        .into(),
                ),
            ],
        ),
    };
    for kids in &buttons {
        for k in kids {
            if let Ok(mut t) = texts.get_mut(*k) {
                if t.0 != want {
                    t.0.clone_from(&want);
                }
            }
        }
    }
}
