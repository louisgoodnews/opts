//! What a control offers and the marker that makes a control "not available yet".

use std::borrow::Cow;

use bevy::prelude::*;

use crate::interaction::state::Disabled;
use crate::interaction::state::DisabledReason;
use super::NotYetPlace;

/// What a control offers right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    /// Works.
    Enabled,
    /// Temporarily unavailable; the reason is shown as tooltip ("Needs a saved game").
    Disabled(Cow<'static, str>),
    /// Does not exist yet; shown with the red marker.
    NotYet(&'static NotYetPlace),
}

impl Availability {
    /// The milestone of a [`Availability::NotYet`].
    pub fn milestone(&self) -> Option<&'static str> {
        match self {
            Self::NotYet(p) => Some(p.milestone),
            _ => None,
        }
    }

    /// Applies the availability to a control (replacing an earlier one).
    pub fn apply(self, entity: &mut EntityCommands) {
        entity.try_remove::<(Disabled, DisabledReason, NotAvailableYet)>();
        match self {
            Self::Enabled => {}
            Self::Disabled(reason) => {
                entity.try_insert((Disabled, DisabledReason(reason)));
            }
            Self::NotYet(place) => {
                entity.try_insert(NotAvailableYet::new(place));
            }
        }
    }
}

/// Marks a control as not available yet. Requires [`Disabled`]; adds the tooltip text and, unless
/// [`NotAvailableYet::without_label`], the red label inside the control.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[require(Disabled)]
pub struct NotAvailableYet {
    /// The registered place.
    pub place: &'static NotYetPlace,
    /// Whether to put the red label inside the control (compact controls place it themselves).
    pub label_inside: bool,
}

impl NotAvailableYet {
    /// A control with the red label inside.
    pub const fn new(place: &'static NotYetPlace) -> Self {
        Self {
            place,
            label_inside: true,
        }
    }

    /// Keeps the tooltip and the disabled look but leaves the label to the screen
    /// ([`spawn_label`]); for small inline controls.
    pub const fn without_label(mut self) -> Self {
        self.label_inside = false;
        self
    }
}

/// The red label node (one style everywhere).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct NotYetLabel;
