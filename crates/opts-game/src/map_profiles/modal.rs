//! The *Map configuration* modal of the Drive wizard (`docs/src/architecture/map-profiles.md` §5):
//! the load profile and its advice, the profiles of the map and the presets, and the buttons
//! *Apply Selected*, *Create New from Current*, *Delete* and *Cancel*.

use bevy::prelude::*;

use super::service::{self, Choice, ProfileView};

/// State of the modal.
#[derive(Resource, Debug, Default)]
pub struct MapConfigMenu {
    /// The modal is showing.
    pub open: bool,
    /// The choice highlighted in the list.
    pub selected: Option<Choice>,
    /// The name typed for a new profile.
    pub name: String,
    /// A line under the title: what the last button did.
    pub status: String,
    view: Option<ProfileView>,
    fingerprint: u64,
    built: u64,
}

/// What a button of the modal does.
#[derive(Component, Clone, Debug)]
pub enum Action {
    Select(Choice),
    Apply { keep: bool },
    Create { chrono: bool },
    Delete,
    Close,
}

#[derive(Component)]
pub struct MapConfigRoot;

#[derive(Component)]
pub struct NameField;

fn fingerprint(v: &Option<ProfileView>, menu: &MapConfigMenu) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    format!("{v:?}|{:?}|{}|{}", menu.selected, menu.status, menu.open).hash(&mut h);
    h.finish()
}

/// Re-reads the view while the modal is open (the counts arrive from a worker).
pub fn refresh_view(world: &mut World) {
    if !world.resource::<MapConfigMenu>().open {
        return;
    }
    let view = service::view(world);
    let mut menu = world.resource_mut::<MapConfigMenu>();
    let fp = fingerprint(&view, &menu);
    menu.view = view;
    menu.fingerprint = fp;
}

mod build;
pub use build::*;

mod actions;
pub use actions::{actions, close_on_cancel, track_name};
