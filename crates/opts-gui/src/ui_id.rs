//! Stable path ids for every interactive element, and the accessibility nodes derived from them.
//!
//! An id is `parent-id/role:slug`: the parent is the nearest ancestor that has an id, the role is
//! the protocol role ([`crate::ui_tree::UiRole`]) and the slug comes from, in this order, the
//! element's [`L10nKey`](crate::l10n::L10nKey) (language independent), its label text, or its
//! position. Screens that want ids that survive a copy change or a language switch set a
//! [`UiId`] themselves (`menu.start`); the automatic ones are for everything else and are made
//! unique with a numeric suffix.

use std::borrow::Cow;

use bevy::prelude::*;

mod assign;
mod sync;

pub use assign::assign_ui_ids;
pub use sync::sync_accessibility;

/// The stable id of a UI element.
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UiId(pub Cow<'static, str>);

impl UiId {
    /// Creates an explicit id.
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }
}

/// Lower-case alphanumeric slug of a label, at most 32 characters.
pub fn slug(label: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in label.chars() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
        if out.chars().count() >= 32 {
            break;
        }
    }
    out.trim_end_matches('-').to_string()
}
/// Registers the id and accessibility systems.
pub fn build(app: &mut App) {
    app.add_systems(PostUpdate, (assign_ui_ids, sync_accessibility).chain());
}

#[cfg(test)]
mod tests;
