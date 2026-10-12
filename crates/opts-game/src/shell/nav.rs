//! Navigation: the [`Navigate`] message, the back stack and deep links.
//!
//! The rail, Home, Help, the control-protocol command `ui.navigate <screen>` and tests all send
//! [`Navigate`]; the shell checks the destination's availability, keeps the back stack and sets
//! the [`AppState`] that hosts the page. No screen re-implements "Escape returns to the menu".

use bevy::prelude::*;
use opts_gui::not_yet::Availability;

use super::registry::ScreenRegistry;
use crate::app_state::AppState;

/// How a navigation treats the back stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavMode {
    /// A flat jump (the rail): the stack is cleared.
    #[default]
    Replace,
    /// A step into a sub-page (*New drive*, *Open this setting*): the current screen is pushed.
    Push,
    /// The shell's own step back; the stack was already popped.
    Pop,
}

/// Request to open a destination: `home`, `settings`, `settings/graphics`, `screen:help`.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct Navigate {
    /// Destination id with an optional deep-link part after the first `/`.
    pub target: String,
    /// Back stack handling.
    pub mode: NavMode,
}

impl Navigate {
    /// A flat jump to `target`.
    pub fn to(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            mode: NavMode::Replace,
        }
    }

    /// A step to `target` that *Back* returns from.
    pub fn push(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            mode: NavMode::Push,
        }
    }
}

/// One step back: pops the stack, or goes Home when it is empty (Home itself stays).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigateBack;

/// A destination the shell refused (not available yet, disabled, unknown).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct NavigationRefused {
    /// The requested id.
    pub screen: String,
    /// Why: the availability of the destination, or `None` for an unknown id.
    pub availability: Option<Availability>,
}

/// The screens *Back* returns to, innermost last.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct BackStack(pub Vec<String>);

/// The deep-link part (`graphics` of `settings/graphics`) for the page that opens next; the page
/// reads and clears it.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct PendingDeepLink(pub Option<String>);

/// Splits `screen:settings/graphics` or `settings/graphics` into id and deep-link part.
pub fn parse_target(target: &str) -> (&str, Option<&str>) {
    let target = target.strip_prefix("screen:").unwrap_or(target).trim();
    match target.split_once('/') {
        Some((id, rest)) if !rest.is_empty() => (id, Some(rest)),
        Some((id, _)) => (id, None),
        None => (target, None),
    }
}

/// Navigation requests collected this frame, applied by [`apply_navigation`] (which needs the
/// whole world to ask a destination about its availability).
#[derive(Resource, Debug, Default)]
pub struct NavQueue(Vec<Navigate>);

/// Moves [`Navigate`] and [`NavigateBack`] messages into the queue (back becomes a pop).
pub fn collect_navigation(
    mut navigate: MessageReader<Navigate>,
    mut back: MessageReader<NavigateBack>,
    state: Res<State<AppState>>,
    registry: Res<ScreenRegistry>,
    mut stack: ResMut<BackStack>,
    mut queue: ResMut<NavQueue>,
) {
    queue.0.extend(navigate.read().cloned());
    for _ in back.read() {
        let current = registry.by_state(*state.get()).map(|s| s.id);
        let target = match stack.0.pop() {
            Some(previous) => previous,
            None if current == Some("home") || current.is_none() => continue,
            None => "home".to_owned(),
        };
        queue.0.push(Navigate {
            target,
            mode: NavMode::Pop,
        });
    }
}

/// Applies the queued navigation: availability check, back stack, deep link, state change.
pub fn apply_navigation(world: &mut World) {
    let requests = std::mem::take(&mut world.resource_mut::<NavQueue>().0);
    for request in requests {
        let (id, deep) = parse_target(&request.target);
        let (id, deep) = (id.to_owned(), deep.map(str::to_owned));
        let Some(descriptor) = world.resource::<ScreenRegistry>().get(&id).copied() else {
            world.write_message(NavigationRefused {
                screen: id,
                availability: None,
            });
            continue;
        };
        let availability = (descriptor.availability)(world);
        let Some(state) = descriptor
            .state
            .filter(|_| availability == Availability::Enabled)
        else {
            world.write_message(NavigationRefused {
                screen: id,
                availability: Some(availability),
            });
            continue;
        };
        let current = world.resource::<State<AppState>>().get().to_owned();
        let current_id = world
            .resource::<ScreenRegistry>()
            .by_state(current)
            .map(|s| s.id.to_owned());
        match request.mode {
            NavMode::Replace => world.resource_mut::<BackStack>().0.clear(),
            NavMode::Push => {
                if let Some(c) = current_id.filter(|c| *c != id) {
                    world.resource_mut::<BackStack>().0.push(c);
                }
            }
            NavMode::Pop => {}
        }
        world.resource_mut::<PendingDeepLink>().0 = deep;
        if current != state {
            world.resource_mut::<NextState<AppState>>().set(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_parse_with_and_without_prefix_and_deep_link() {
        assert_eq!(parse_target("home"), ("home", None));
        assert_eq!(parse_target("screen:settings"), ("settings", None));
        assert_eq!(
            parse_target("screen:settings/graphics"),
            ("settings", Some("graphics"))
        );
        assert_eq!(parse_target("settings/"), ("settings", None));
        assert_eq!(
            parse_target("settings/graphics/filter"),
            ("settings", Some("graphics/filter"))
        );
    }
}
