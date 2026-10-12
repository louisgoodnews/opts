//! [`ThemePlugin`]: registers the theme resources and the systems that keep them in step with the
//! settings, the window and the clock.

use bevy::prelude::*;

use super::{
    instrument::InstrumentVariant,
    mode::{ThemeKind, ThemeMode},
    motion::{drive_background_transitions, sync_motion_preference, MotionPreference},
    runtime::{
        resolve_theme, track_system_theme, SystemColorScheme, ThemeClock, ThemePreferences,
        ThemeRuntime, ThemeSwitchLock,
    },
    scale::{apply_ui_scale, clamp_text_size, update_layout_class, MinTextSize, UiScalePercent},
    spacing::{Density, LayoutClass},
    theme::MaterialTheme,
};
use crate::tokens::sync_elevation_shadows;

/// Registers the theme state and its systems (added by [`OptsGuiPlugin`](crate::OptsGuiPlugin)).
///
/// Resources: [`ThemePreferences`], [`ThemeSwitchLock`], [`SystemColorScheme`], [`ThemeClock`],
/// [`UiScalePercent`], [`MinTextSize`], [`Density`], [`LayoutClass`], [`MotionPreference`],
/// [`InstrumentVariant`]. Theme resolution runs in `PreUpdate`, before any widget system reads
/// the theme.
#[derive(Default)]
pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<ThemePreferences>() {
            // A theme inserted before the plugin (tests, other apps) is kept: the preferences
            // start out matching it instead of replacing it with the default.
            let mut prefs = ThemePreferences::default();
            match app
                .world()
                .get_resource::<MaterialTheme>()
                .map(|t| t.kind())
            {
                Some(ThemeKind::Light) => prefs.mode = ThemeMode::Light,
                Some(ThemeKind::HighContrast) => prefs.high_contrast = true,
                _ => {}
            }
            app.insert_resource(prefs);
        }
        app.add_message::<bevy::window::WindowThemeChanged>()
            .init_resource::<ThemeSwitchLock>()
            .init_resource::<SystemColorScheme>()
            .init_resource::<ThemeClock>()
            .init_resource::<ThemeRuntime>()
            .init_resource::<UiScalePercent>()
            .init_resource::<MinTextSize>()
            .init_resource::<Density>()
            .init_resource::<LayoutClass>()
            .init_resource::<MotionPreference>()
            .init_resource::<InstrumentVariant>()
            .init_resource::<UiScale>();
        app.add_systems(
            PreUpdate,
            (
                track_system_theme,
                resolve_theme,
                sync_motion_preference,
                apply_ui_scale,
            )
                .chain(),
        );
        app.add_systems(
            Update,
            (
                update_layout_class,
                drive_background_transitions,
                sync_elevation_shadows,
                crate::fonts::apply_ui_font,
                clamp_text_size.after(crate::tokens::sync_typestyle_tokens),
            ),
        );
    }
}
