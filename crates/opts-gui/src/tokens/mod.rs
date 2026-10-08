//! Reference tokens and token components for Material Design 3.
//!
//! Provides the marker components that link entities to theme tokens
//! ([`BackgroundRole`], [`ForegroundRole`], [`BorderRole`], [`RadiusToken`],
//! [`ElevationToken`], [`TypeStyleToken`]) and their reactive synchronization systems.

pub mod colors;
pub mod components;

pub use components::{
    sync_background_roles, sync_border_roles, sync_foreground_roles, sync_radius_tokens,
    sync_typestyle_tokens, BackgroundRole, BorderRole, ElevationToken, ForegroundRole, RadiusToken,
    TypeStyleToken,
};
