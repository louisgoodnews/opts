//! Material Design 3 Toast / Snackbar notification widget: components, severity and the systems.

use bevy::prelude::*;

use crate::theme::color::ColorRole;

mod builder;
mod spawn;
mod systems;

pub use builder::ToastBuilder;
pub use systems::{toast_action_click_system, toast_timer_system, ToastPlugin};

/// Marker component for an M3 Toast root entity.
#[derive(Component, Debug, Clone)]
pub struct M3Toast {
    pub message: String,
    pub timer: Option<Timer>,
}

/// How serious a toast is; sets its colours and how long it stays.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToastSeverity {
    /// Something finished (4 s).
    #[default]
    Info,
    /// Something to be aware of (10 s).
    Warning,
    /// Something failed (10 s). An error is never *only* a toast: its owner also shows it in a
    /// durable place (inline, banner or dialog).
    Error,
}

impl ToastSeverity {
    /// Seconds a toast of this severity stays by default.
    pub const fn default_seconds(self) -> f32 {
        match self {
            Self::Info => 4.0,
            Self::Warning | Self::Error => 10.0,
        }
    }

    pub(super) fn roles(self) -> (ColorRole, ColorRole) {
        match self {
            Self::Info => (ColorRole::InverseSurface, ColorRole::InverseOnSurface),
            Self::Warning => (ColorRole::TertiaryContainer, ColorRole::OnTertiaryContainer),
            Self::Error => (ColorRole::ErrorContainer, ColorRole::OnErrorContainer),
        }
    }

    /// Colour of the action label: the accent of the inverse surface for information, the text
    /// colour of the container otherwise (the inverse accent is unreadable on those).
    pub(super) fn action_role(self) -> ColorRole {
        match self {
            Self::Info => ColorRole::InversePrimary,
            Self::Warning | Self::Error => self.roles().1,
        }
    }
}
/// Spawn order of a toast (stacking order; the newest has the highest number).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ToastSeq(pub u64);

/// Toasts with the same key are merged while one is alive (a counter shows how often).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct ToastDedupe(pub String);

/// How many identical notifications this toast stands for.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToastCount(pub u32);

/// A toast waiting for a free slot (at most [`crate::components::toast_stack::MAX_VISIBLE`] show
/// at once); its timer does not run.
#[derive(Component, Debug, Clone, Copy)]
pub struct ToastQueued;
/// Marker component for the optional action button on a toast.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToastActionButton(pub Entity);
#[cfg(test)]
mod tests;
