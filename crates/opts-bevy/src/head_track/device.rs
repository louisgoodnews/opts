//! The device boundary of head tracking. A [`HeadPoseSource`] reports poses; this build opens the
//! no-op source, which never reports one. A TrackIR backend implements the same trait.

use super::pose::HeadPose;

/// A failure of a tracking device.
#[derive(Debug, thiserror::Error)]
pub enum HeadPoseError {
    /// The device stopped answering, or its library could not be read.
    #[error("head-tracking device failed: {0}")]
    Device(String),
}

/// A source of head poses.
pub trait HeadPoseSource: Send + Sync {
    /// A short name of the device, for the log.
    fn name(&self) -> &'static str;

    /// The newest pose, or `Ok(None)` when the device has no new frame.
    fn poll(&mut self) -> Result<Option<HeadPose>, HeadPoseError>;
}

/// The source of a build without a device backend: it never reports a pose.
#[derive(Debug, Default)]
pub struct NoopHeadPoseSource;

impl HeadPoseSource for NoopHeadPoseSource {
    fn name(&self) -> &'static str {
        "no device backend"
    }

    fn poll(&mut self) -> Result<Option<HeadPose>, HeadPoseError> {
        Ok(None)
    }
}

/// The source this build opens.
// TODO(owner): the TrackIR backend is not written. It would bind the NaturalPoint NP client
// library, which is a Windows DLL that a Linux build cannot load; that needs a decision on how
// to load it without a new native dependency (ISS-045, M-0166).
#[must_use]
pub fn open_default_source() -> Box<dyn HeadPoseSource> {
    Box::new(NoopHeadPoseSource)
}
