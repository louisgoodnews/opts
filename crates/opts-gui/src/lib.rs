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
// Bevy 0.20 deprecated `Interaction` (now `Hovered` + `Pressed`); the migration is the follow-up
// of M-0047 (docs/src/architecture/bevy-0-20-dlss.md). Until then the legacy state keeps working.
#![allow(deprecated)]

pub mod components;
pub mod fonts;
pub mod icons;
pub mod interaction;
pub mod layout;
pub mod plugin;
pub mod prelude;
pub mod theme;
pub mod tokens;

pub use plugin::OptsGuiPlugin;
