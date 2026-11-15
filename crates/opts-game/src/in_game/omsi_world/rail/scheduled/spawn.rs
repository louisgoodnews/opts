//! The steps of [`spawn_scheduled_trains`]: which runs are candidates, where a run's train starts
//! and which way it drives, whether it may appear, and the train with its stops.

mod candidates;
mod context;
mod find;
mod track;
mod train;

pub(super) use candidates::*;
pub(super) use context::*;
pub(super) use find::*;
pub(super) use track::*;
pub(super) use train::*;
