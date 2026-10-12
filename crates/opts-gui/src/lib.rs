//! Material Design 3 GUI widget library for the Open Public Transport Simulator (OPTS).
//!
//! `opts-gui` provides a modular, token-driven user interface toolkit built natively
//! on Bevy UI (`bevy_ui`).
//!
//! # Features
//! - **Material Design 3 Tokens:** Color, typography, spacing, shape, elevation, and motion.
//! - **Theming & Accessibility:** Preconfigured light and dark schemes compliant with WCAG AA.
//! - **Reactive State Layer:** Decoupled tokens and interaction states for all components.
//! - **Ergonomic Builders:** High-level fluent builders for UI layout and widgets.

#![deny(unsafe_code)]
pub mod actions;
pub mod components;
pub mod fonts;
pub mod icons;
pub mod interaction;
pub mod l10n;
pub mod layout;
pub mod not_yet;
pub mod plugin;
pub mod prelude;
pub mod testing;
pub mod theme;
pub mod tokens;
pub mod ui_id;
pub mod ui_tree;

pub use plugin::OptsGuiPlugin;
