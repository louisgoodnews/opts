//! Application shell contract: where screens put their content.
//!
//! The shell (navigation rail, banners, overlays; see `docs/src/roadmap/phase-9/flow.md`) owns
//! one *content area*. Every screen builds its content with [`spawn_screen_root`], which places
//! it inside that area when the shell is active and falls back to a full-window root otherwise
//! (menus before the shell exists, headless tests). Screens therefore never care whether the
//! rail is visible, and the shell can be introduced without rewriting them.

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
}
