//! Localisation (scaffold; the message tables and `Localization` arrive with batch B6).

use bevy::prelude::*;

/// The message key a text element was built from; makes ids and tests independent of the language.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct L10nKey(pub &'static str);

/// The key of an element or of its first descendant that has one.
pub fn key_of(world: &World, entity: Entity) -> Option<String> {
    let e = world.get_entity(entity).ok()?;
    if let Some(k) = e.get::<L10nKey>() {
        return Some(k.0.to_string());
    }
    for child in e.get::<Children>().into_iter().flat_map(|c| c.iter()) {
        if let Some(k) = key_of(world, child) {
            return Some(k);
        }
    }
    None
}
