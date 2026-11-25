//! The per-frame head-tracking systems: open the device while tracking is on, read its pose and
//! apply a centre request.

use bevy::prelude::*;

use super::device::open_default_source;
use super::state::{HeadTrackSettings, HeadTrackState};

/// Opens the device when head tracking is switched on, and closes it when it is switched off.
pub fn manage_source(settings: Res<HeadTrackSettings>, mut state: ResMut<HeadTrackState>) {
    if !settings.enabled {
        if state.source.take().is_some() {
            tracing::debug!("head-tracking source closed");
        }
        return;
    }
    if state.source.is_none() {
        let source = open_default_source();
        tracing::debug!(device = source.name(), "head-tracking source opened");
        state.source = Some(source);
        state.has_pose = false;
        state.error_reported = false;
    }
}

/// Reads the newest pose of the open device. A device error is logged once per run of errors.
pub fn poll_head_pose(mut state: ResMut<HeadTrackState>) {
    let state = &mut *state;
    let Some(source) = state.source.as_mut() else {
        return;
    };
    match source.poll() {
        Ok(Some(pose)) if pose.is_finite() => {
            tracing::trace!(
                yaw = pose.yaw,
                pitch = pose.pitch,
                roll = pose.roll,
                "head pose"
            );
            if !state.has_pose {
                tracing::debug!(
                    device = source.name(),
                    yaw = pose.yaw,
                    pitch = pose.pitch,
                    roll = pose.roll,
                    "first head pose received"
                );
                state.has_pose = true;
            }
            state.latest = pose;
            state.error_reported = false;
        }
        Ok(_) => {}
        Err(err) => {
            if !state.error_reported {
                tracing::debug!(error = %err, "head-tracking device error");
                state.error_reported = true;
            }
        }
    }
}

/// Makes the newest pose the centre when a centre was requested.
pub fn apply_centre(mut state: ResMut<HeadTrackState>) {
    if !std::mem::take(&mut state.centre_requested) {
        return;
    }
    state.centre = state.latest;
    tracing::debug!(
        yaw = state.centre.yaw,
        pitch = state.centre.pitch,
        "head pose centred"
    );
}
