//! The data of the scheduler: the stops and plan of a rail run, and the system parameters the
//! scheduler reads and changes.

use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};

use super::*;

/// A stop of a train's run.
#[derive(Debug, Clone)]
pub(in super::super) struct RailStop {
    pub(in super::super) station: u32,
    pub(in super::super) lane: LaneId,
    /// Where the head stands when the train is at the stop (m along the lane).
    pub(in super::super) s: f32,
    pub(in super::super) dwell_s: f32,
    /// Earliest departure (clock, s).
    pub(in super::super) depart_not_before: f64,
    /// Scheduled arrival (clock, s).
    pub(in super::super) scheduled: f64,
    pub(in super::super) arrived_at: Option<f64>,
}

/// The run a scheduled train drives and the stops still ahead.
#[derive(Debug, Clone)]
pub(in super::super) struct RailPlan {
    pub(in super::super) run: RunRef,
    pub(in super::super) stops: VecDeque<RailStop>,
    pub(in super::super) retire_at: Option<f64>,
    /// Where the doors are (`AI_Scheduled_AtStation`, as for road vehicles).
    pub(in super::super) doors: Doors,
    /// Seconds since the units' scripts last ran a frame.
    pub(in super::super) since: f32,
}
/// The resources and the world the scheduler reads and changes.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TrainWorld<'w> {
    pub(super) time: Res<'w, Time<Real>>,
    pub(super) job: Option<ResMut<'w, WorldLoadJob>>,
    pub(super) env: Option<Res<'w, opts_bevy::SessionEnvironment>>,
    pub(super) net: Res<'w, RailNetwork>,
    pub(super) pool: Res<'w, RailAiPool>,
    pub(super) sched: ResMut<'w, ScheduledTraffic>,
}

/// The asset stores a new train's meshes go into.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TrainAssets<'w> {
    pub(super) meshes: ResMut<'w, Assets<Mesh>>,
    pub(super) materials: ResMut<'w, Assets<StandardMaterial>>,
    pub(super) images: ResMut<'w, Assets<Image>>,
}

/// The scheduler's own memory between runs of the system.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TrainLocals<'s> {
    pub(super) timer: Local<'s, f32>,
    pub(super) tries: Local<'s, HashMap<RunRef, u32>>,
    pub(super) not_rail: Local<'s, HashSet<String>>,
}
