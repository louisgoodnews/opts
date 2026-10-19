//! `opts-l10n`: localisation without the engine: the Fluent-subset parser, language packs, checks and the
//! JSON interchange form. The game (`opts-gui`) and `opts-tools l10n` share this code, so what
//! the tool validates is what the game loads.
//!
//! See `docs/src/architecture/localisation-packs.md`.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod builtin;
pub mod check;
pub mod fluent;
pub mod interchange;
pub mod pack;

pub use check::{Issue, Reference, Severity, Stats};
pub use pack::{Direction, Pack, PackMeta};
