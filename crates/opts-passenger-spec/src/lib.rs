//! # `opts-passenger-spec`: the passenger files as the simulation's specs
//!
//! Split out of `opts-game`'s `passengers` module (ISS-064). The conversions are pure functions
//! from the parsed `passengercabin.cfg`, `paths.cfg` and `.otp` to the specs of `opts-sim`, so
//! `opts-game` and `opts-world-load` share them without depending on each other.
//!
//! `opts-game` re-exports them at their old paths (`opts_game::passengers::unit_spec` and so on).

pub mod convert;

pub use convert::{cabin_spec, paths_spec, ticket_book, unit_spec};
