//! The screen registry: one descriptor per destination, from which the rail is generated.
//!
//! Adding a screen is one module and one [`ScreenAppExt::register_screen`] call
//! (`docs/src/roadmap/phase-9/expandability.md` section 4). The page itself is still built by the
//! screen's own `OnEnter` system of the [`AppState`] named in the descriptor, with
//! [`crate::shell::spawn_screen_root`].

use bevy::prelude::*;
use opts_gui::not_yet::{self, Availability};

use super::{icons, keys};
use crate::app_state::AppState;

/// The group a destination belongs to in the rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScreenGroup {
    /// The main destinations (at most five).
    Main,
    /// Utility destinations at the bottom of the rail.
    Utility,
}

/// Decides what a destination offers right now (enabled, temporarily disabled with a reason, or
/// not available yet).
pub type AvailabilityFn = fn(&World) -> Availability;

/// One destination of the shell.
#[derive(Debug, Clone, Copy)]
pub struct ScreenDescriptor {
    /// Stable id for deep links, tests and `ui.navigate` (`settings`).
    pub id: &'static str,
    /// Message key of the label.
    pub title_key: &'static str,
    /// Material Icons codepoint (see [`icons`]).
    pub icon: &'static str,
    /// Rail group.
    pub group: ScreenGroup,
    /// Sort order inside the group.
    pub order: u16,
    /// What the destination offers right now.
    pub availability: AvailabilityFn,
    /// The state that hosts the page; `None` for a destination that does not exist yet.
    pub state: Option<AppState>,
}

/// The registered destinations.
#[derive(Resource, Debug, Default, Clone)]
pub struct ScreenRegistry {
    screens: Vec<ScreenDescriptor>,
}

impl ScreenRegistry {
    /// Adds a destination; an earlier one with the same id is replaced.
    pub fn register(&mut self, screen: ScreenDescriptor) {
        self.screens.retain(|s| s.id != screen.id);
        self.screens.push(screen);
    }

    /// The destination with `id`.
    pub fn get(&self, id: &str) -> Option<&ScreenDescriptor> {
        self.screens.iter().find(|s| s.id == id)
    }

    /// The destination hosted by `state`.
    pub fn by_state(&self, state: AppState) -> Option<&ScreenDescriptor> {
        self.screens.iter().find(|s| s.state == Some(state))
    }

    /// The destinations of `group`, in rail order.
    pub fn group(&self, group: ScreenGroup) -> Vec<&ScreenDescriptor> {
        let mut found: Vec<_> = self.screens.iter().filter(|s| s.group == group).collect();
        found.sort_by_key(|s| (s.order, s.id));
        found
    }

    /// All destinations.
    pub fn all(&self) -> &[ScreenDescriptor] {
        &self.screens
    }
}

/// `App::register_screen`.
pub trait ScreenAppExt {
    /// Registers a destination in the shell's [`ScreenRegistry`].
    fn register_screen(&mut self, screen: ScreenDescriptor) -> &mut Self;
}

impl ScreenAppExt for App {
    fn register_screen(&mut self, screen: ScreenDescriptor) -> &mut Self {
        self.world_mut()
            .get_resource_or_insert_with(ScreenRegistry::default)
            .register(screen);
        self
    }
}

fn enabled(_: &World) -> Availability {
    Availability::Enabled
}
fn library(_: &World) -> Availability {
    Availability::NotYet(&not_yet::NAV_LIBRARY)
}
fn workshop(_: &World) -> Availability {
    Availability::NotYet(&not_yet::NAV_WORKSHOP)
}

/// The destinations of the shell itself (Home, Drive, Library, Workshop, Settings, Help).
pub fn builtin_screens() -> [ScreenDescriptor; 6] {
    let screen = |id, title_key, icon, group, order, availability, state| ScreenDescriptor {
        id,
        title_key,
        icon,
        group,
        order,
        availability,
        state,
    };
    use ScreenGroup::{Main, Utility};
    [
        screen(
            "home",
            keys::NAV_HOME,
            icons::HOME,
            Main,
            0,
            enabled,
            Some(AppState::MainMenu),
        ),
        screen(
            "drive",
            keys::NAV_DRIVE,
            icons::DRIVE,
            Main,
            10,
            enabled,
            Some(AppState::VehicleSelection),
        ),
        screen(
            "library",
            keys::NAV_LIBRARY,
            icons::LIBRARY,
            Main,
            20,
            library,
            None,
        ),
        screen(
            "workshop",
            keys::NAV_WORKSHOP,
            icons::WORKSHOP,
            Main,
            30,
            workshop,
            None,
        ),
        screen(
            "settings",
            keys::NAV_SETTINGS,
            icons::SETTINGS,
            Utility,
            0,
            enabled,
            Some(AppState::Settings),
        ),
        screen(
            "help",
            keys::NAV_HELP,
            icons::HELP,
            Utility,
            10,
            enabled,
            Some(AppState::Help),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtin_rail_has_four_main_and_two_utility_entries_in_order() {
        let mut registry = ScreenRegistry::default();
        for s in builtin_screens() {
            registry.register(s);
        }
        let ids = |g| registry.group(g).iter().map(|s| s.id).collect::<Vec<_>>();
        assert_eq!(
            ids(ScreenGroup::Main),
            ["home", "drive", "library", "workshop"]
        );
        assert_eq!(ids(ScreenGroup::Utility), ["settings", "help"]);
        assert!(registry.group(ScreenGroup::Main).len() <= 5, "UI-P02");
    }

    #[test]
    fn a_registration_with_an_existing_id_replaces_it() {
        let mut registry = ScreenRegistry::default();
        let mut home = builtin_screens()[0];
        registry.register(home);
        home.order = 99;
        registry.register(home);
        assert_eq!(registry.all().len(), 1);
        assert_eq!(registry.get("home").map(|s| s.order), Some(99));
        assert_eq!(
            registry.by_state(AppState::MainMenu).map(|s| s.id),
            Some("home")
        );
    }
}
