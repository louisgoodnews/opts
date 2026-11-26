//! The rail-bound vehicle state of the open-world session (split out of `opts-game-state`, ISS-084):
//! the box of a vehicle part ([`body`]), the composed units of a vehicle and the player's rail
//! vehicle ([`rail`]). The physics live in `opts_sim::rail`; `opts-game-state` re-exports both
//! modules at their old paths.

pub mod body;
pub mod rail;
