//! Shell navigation: the screen registry, the navigation rail frame, the back stack, Esc and banners.
//!
//! Split out of `opts-game` (ISS-068 stage 2). The shell (`docs/src/roadmap/phase-9/flow.md`,
//! `screens.md`) owns one *content area*. Every screen builds its content with [`spawn_screen_root`],
//! which places it inside that area when the shell is active and falls back to a full-window root
//! otherwise (during a drive, headless tests). Screens therefore never care whether the rail is
//! visible.
//!
//! * [`registry`]: [`ScreenDescriptor`]s; the rail is generated from them.
//! * [`nav`]: [`Navigate`], the back stack, deep links (`screen:settings/graphics`).
//! * [`esc`]: the one implementation of Esc / Back.
//! * [`frame`]: the rail and the content area; [`banner`]: durable problems of a screen.
//!
//! The plugin that registers the systems, the footer and the notification service stay in
//! `opts-game`, which re-exports this crate at the old paths.

pub mod banner;
pub mod esc;
pub mod frame;
pub mod nav;
pub mod registry;

pub use opts_shell_icons as icons;
pub use opts_shell_keys as keys;

use bevy::prelude::*;

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
