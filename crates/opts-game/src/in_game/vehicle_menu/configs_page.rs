//! The *Configurations* page of the Alt menu (M-0077): the saved bus configurations, each with
//! *Spawn* (builds it as a new session vehicle at the session's start point) and *Delete* (asks
//! with a second press), and *Save current vehicle* for the active session vehicle.

mod actions;
mod rows;
mod view;

pub(super) use actions::{commit_rename, delete, save_active, spawn, start_rename};
pub(super) use rows::{load_view, ConfigsView};
pub(super) use view::build_configs;

#[cfg(test)]
use super::*;
#[cfg(test)]
use crate::bus_config::{BusConfiguration, BusConfigurations};
#[cfg(test)]
use crate::new_game_screen::configs::ConfigLibrary;
#[cfg(test)]
use actions::free_name;
#[cfg(test)]
use rows::rows_of;

#[cfg(test)]
mod tests;
