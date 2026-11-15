//! Where a placement came from, the counters around it and the transform and notes it records.

use bevy::prelude::*;
use opts_bevy::scene_trace::{MeshNote, TraceTransform};

use super::*;

/// Where in the map a placement came from.
#[derive(Clone, Copy)]
pub(in crate::in_game::omsi_world) struct Site<'a> {
    /// `objects` or `attachments`.
    pub list: &'static str,
    /// Index in that list.
    pub index: usize,
    /// For a parked car: the marker object it stands for.
    pub marker: Option<&'a str>,
}

/// Counters before a placement, compared with the counters after it.
#[derive(Clone, Copy, Default)]
pub(in crate::in_game::omsi_world) struct Counters {
    pub(super) loaded: usize,
    pub(super) failed: usize,
    pub(super) no_geometry: usize,
    pub(super) protected: usize,
    pub(super) hidden: usize,
}

impl Counters {
    /// Reads the counters.
    pub fn of(job: &WorldLoadJob, stats: &WorldStats) -> Self {
        Self {
            loaded: stats.objects_loaded,
            failed: stats.objects_failed,
            no_geometry: stats.objects_no_geometry,
            protected: stats.objects_protected,
            hidden: job.cache.stats.hidden_parts,
        }
    }
}

pub(in crate::in_game::omsi_world) fn to_transform(t: &Transform) -> TraceTransform {
    TraceTransform {
        t: t.translation.to_array(),
        q: t.rotation.to_array(),
        s: t.scale.to_array(),
    }
}

pub(in crate::in_game::omsi_world) fn notes(frame: &Frame) -> Vec<MeshNote> {
    frame.meshes.iter().map(|p| st::note_for(p)).collect()
}
