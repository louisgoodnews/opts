//! Head tracking (ISS-045, M-0166): a head pose from a tracking device turns the look of the
//! driver's view (key 2). Off by default. The device sits behind [`HeadPoseSource`], so the build
//! without a backend simply never reports a pose.
//!
//! The game writes [`HeadTrackSettings`] from its settings (`opts-game`, `head_track_settings.rs`).

pub mod device;
pub mod pose;
pub mod state;
mod systems;

use bevy::prelude::*;

pub use device::{open_default_source, HeadPoseError, HeadPoseSource, NoopHeadPoseSource};
pub use pose::{HeadPose, HEAD_PITCH_LIMIT_DEG, HEAD_YAW_LIMIT_DEG};
pub use state::{HeadTrackSettings, HeadTrackState, DEFAULT_GAIN, MAX_GAIN};

/// Registers the head-tracking resources and the systems that read the device.
#[derive(Default)]
pub struct HeadTrackPlugin;

impl Plugin for HeadTrackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeadTrackSettings>()
            .init_resource::<HeadTrackState>()
            .add_systems(
                Update,
                (
                    systems::manage_source,
                    systems::poll_head_pose,
                    systems::apply_centre,
                )
                    .chain(),
            );
    }
}
