//! Application shell: navigation rail, screen registry, back stack, Esc and dialogs.
//!
//! The shell (`docs/src/roadmap/phase-9/flow.md`, `screens.md`) owns one *content area*. Every
//! screen builds its content with [`spawn_screen_root`], which places it inside that area when
//! the shell is active and falls back to a full-window root otherwise (during a drive, headless
//! tests). Screens therefore never care whether the rail is visible.
//!
//! * [`registry`]: [`ScreenDescriptor`]s; the rail is generated from them.
//! * [`nav`]: [`Navigate`], the back stack, deep links (`screen:settings/graphics`).
//! * [`esc`]: the one implementation of Esc / Back.
//! * [`dialog`]: confirmation dialogs; [`footer`]: theme toggle and quit.

pub mod banner;
pub mod calendar;
pub mod dialog;
pub mod esc;
pub mod footer;
pub mod frame;
pub mod icons;
pub mod keys;
pub mod nav;
pub mod notify;
pub mod registry;

use bevy::prelude::*;

use crate::app_state::AppState;

/// The shell's content area, once the shell has been spawned.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct ShellContent(pub Option<Entity>);

/// Marker on the root node of a screen built by [`spawn_screen_root`].
#[derive(Component, Debug, Clone, Copy)]
pub struct ScreenRoot;

/// Spawns the root node of a screen and returns it; attach the screen's children to it and
/// despawn it (recursively) when the screen is left.
///
/// With an active shell the root is a child of the content area and fills it; without a shell it
/// is a full-window root.
pub fn spawn_screen_root(
    commands: &mut Commands,
    shell: &ShellContent,
    name: &'static str,
) -> Entity {
    let node = Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        flex_direction: FlexDirection::Column,
        ..default()
    };
    let root = commands.spawn((Name::new(name), ScreenRoot, node)).id();
    if let Some(content) = shell.0 {
        commands.entity(content).add_child(root);
    }
    root
}

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
mod tests {
    use super::*;

    #[test]
    fn without_a_shell_the_screen_root_has_no_parent() {
        let mut world = World::new();
        let root = {
            let mut commands = world.commands();
            spawn_screen_root(&mut commands, &ShellContent::default(), "test")
        };
        world.flush();
        assert!(world.get::<ChildOf>(root).is_none());
        assert!(world.get::<ScreenRoot>(root).is_some());
    }

    #[test]
    fn with_a_shell_the_screen_root_lives_in_the_content_area() {
        let mut world = World::new();
        let area = world.spawn(Node::default()).id();
        let root = {
            let mut commands = world.commands();
            spawn_screen_root(&mut commands, &ShellContent(Some(area)), "test")
        };
        world.flush();
        assert_eq!(world.get::<ChildOf>(root).map(ChildOf::parent), Some(area));
    }

    #[test]
    fn every_shell_key_exists_in_english_and_german_and_nothing_else_does() {
        let mut loc = opts_gui::l10n::Localization::new("en");
        extend_localization_into(&mut loc);
        for key in keys::ALL {
            assert!(loc.has("en", key), "shell-en.ftl lacks `{key}`");
            assert!(loc.has("de", key), "shell-de.ftl lacks `{key}`");
        }
        for table in [
            include_str!("l10n/shell-en.ftl"),
            include_str!("l10n/shell-de.ftl"),
        ] {
            for line in table
                .lines()
                .filter(|l| l.contains(" = ") && !l.starts_with(['#', ' ']))
            {
                let key = line.split(" = ").next().unwrap_or_default();
                assert!(
                    keys::ALL.contains(&key)
                        || opts_bevy::reporting::keys::ALL
                            .iter()
                            .any(|(k, _)| *k == key),
                    "`{key}` is in a table but not in keys::ALL"
                );
            }
        }
    }
}
