//! `opts-core`: a **temporary facade** over the crates the old `opts-core` was split into
//! ([ADR-0014](../../docs/src/adr/0014-data-model-separation.md), Phase 13).
//!
//! Every path that used to live here still resolves, so no `use` statement had to change when the
//! code moved:
//!
//! | Old path | Now lives in |
//! | :--- | :--- |
//! | `opts_core::formats::*`, `opts_core::setvars` | `opts-omsi` (the legacy OMSI readers) |
//! | `opts_core::l10n::*` | `opts-l10n` (localisation, no OMSI content) |
//!
//! New code should name the real crate. The facade goes when the last consumer is migrated; the
//! ratchet `docs/dev/omsi-uses-known.txt` (checked by `scripts/omsi_uses.py`) counts the files of
//! the engine crates that still reach OMSI-shaped types and may only fall.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub use opts_l10n as l10n;
pub use opts_omsi::{formats, setvars};
