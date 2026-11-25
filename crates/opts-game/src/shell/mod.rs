//! Application shell: navigation rail, screen registry, back stack, Esc and dialogs.
//!
//! The navigation (the registry, the rail frame, the back stack, Esc and banners) is the crate
//! `opts-shell-nav` (ISS-068 stage 2), re-exported here at the old paths. This module keeps the
//! plugin that registers its systems, the footer (theme toggle and quit) and the notification
//! service, which use the game's settings and modal state.
//!
//! * [`registry`]: [`ScreenDescriptor`]s; the rail is generated from them.
//! * [`nav`]: [`Navigate`], the back stack, deep links (`screen:settings/graphics`).
//! * [`esc`]: the one implementation of Esc / Back.
//! * [`dialog`]: confirmation dialogs; [`footer`]: theme toggle and quit.

pub mod footer;
pub use opts_shell_keys as keys;
pub mod notify;

// Leaves split out as crates (ISS-068 stage 1); the old module paths stay.
pub use opts_shell_calendar as calendar;
pub use opts_shell_dialog as dialog;
pub use opts_shell_icons as icons;

// Navigation split out as crate `opts-shell-nav` (ISS-068 stage 2); the old module paths stay.
pub use opts_shell_nav::{banner, esc, frame, nav, registry, spawn_screen_root, ScreenRoot, ShellContent};

use bevy::prelude::*;

use crate::app_state::AppState;

pub use nav::{BackStack, NavMode, Navigate, NavigateBack, NavigationRefused, PendingDeepLink};
pub use registry::{ScreenAppExt, ScreenDescriptor, ScreenGroup, ScreenRegistry};
/// Adds the shell's messages, registry, frame and systems.
pub struct ShellPlugin;

/// Merges the shell's message tables into the localisation.
fn extend_localization(world: &mut World) {
    let mut loc = world.get_resource_or_insert_with(opts_gui::l10n::Localization::detect);
    extend_localization_into(&mut loc);
}

/// Merges the tables into `loc`.
fn extend_localization_into(loc: &mut opts_gui::l10n::Localization) {
    for (locale, source) in [
        ("en", include_str!("l10n/shell-en.ftl")),
        ("de", include_str!("l10n/shell-de.ftl")),
    ] {
        for problem in loc.extend(locale, source) {
            tracing::warn!("shell message table ({locale}): {problem}");
        }
    }
}

impl Plugin for ShellPlugin {
    fn build(&self, app: &mut App) {
        extend_localization(app.world_mut());
        for screen in registry::builtin_screens() {
            app.register_screen(screen);
        }
        app.init_resource::<ShellContent>()
            .init_resource::<BackStack>()
            .init_resource::<PendingDeepLink>()
            .init_resource::<nav::NavQueue>()
            .add_message::<Navigate>()
            .add_message::<NavigateBack>()
            .add_message::<NavigationRefused>();
        banner::build(app);
        dialog::build(app);
        app.add_systems(Update, notify::refusal_toasts);
        esc::build(app);

        app.add_systems(Startup, frame::spawn_shell_frame);
        // The frame is hidden (and `ShellContent` released) before the drive's own screens
        // are built, and comes back before Home is built again.
        for exited in [
            AppState::Loading,
            AppState::MainMenu,
            AppState::VehicleSelection,
            AppState::Settings,
            AppState::Help,
        ] {
            app.add_systems(
                OnTransition {
                    exited,
                    entered: AppState::InGame,
                },
                frame::hide_shell,
            );
        }
        app.add_systems(OnExit(AppState::InGame), frame::show_shell);

        app.add_systems(
            Update,
            (
                frame::rail_click_system,
                nav::collect_navigation,
                nav::apply_navigation,
            )
                .chain()
                .after(esc::shell_back_system),
        )
        .add_systems(
            Update,
            (
                frame::sync_rail_layout,
                frame::sync_rail_selection,
                frame::refresh_rail_availability,
                frame::sync_rail_not_yet_labels,
                footer::sync_theme_toggle,
                footer::quit_click_system,
                footer::quit_choice_system,
            ),
        )
        .add_systems(
            Update,
            footer::theme_toggle_system
                .run_if(not(in_state(AppState::InGame)))
                .run_if(opts_bevy::reporting::not_modal_active),
        );
    }
}
#[cfg(test)]
mod tests;
