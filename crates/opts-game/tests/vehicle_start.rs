//! Start procedures of the reference vehicles (M-0141): the vehicles' own scripts, driven the way
//! a player drives them (hot-spot press/drag/release, keys), from a cold start to a running engine.
//! The procedures are written down in `docs/src/quality/vehicle-start-procedures.md`.
//!
//! Needs an OMSI installation (`OMSI_PATH`); skips without one.

#[path = "vehicle_start/rig.rs"]
mod rig;
mod vehicle_rig;

use opts_core::formats::vehicles::bus::BusManifest;
use rig::{click, start_with_setup, tap};
use vehicle_rig::{load, run};

#[path = "vehicle_start/census.rs"]
mod census;
#[path = "vehicle_start/census_tram.rs"]
mod census_tram;
#[path = "vehicle_start/controls_report.rs"]
mod controls_report;
#[path = "vehicle_start/hybrid.rs"]
mod hybrid;
#[path = "vehicle_start/starts.rs"]
mod starts;
