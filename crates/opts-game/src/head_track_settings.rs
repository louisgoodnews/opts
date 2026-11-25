//! The head-tracking settings (ISS-045, M-0166): whether the driver's view follows the head, and
//! the gain. [`sync_head_track_settings`] copies them into `HeadTrackSettings` of `opts-bevy`.

use bevy::prelude::*;
use opts_bevy::head_track::{HeadTrackSettings, DEFAULT_GAIN, MAX_GAIN};

use crate::settings::GameSettings;
use crate::settings_registry::descriptor::{
    Category as C, SettingDescriptor as D, SettingKind as K,
};

/// `controls.head_tracking`: `true` or `false` (off by default).
pub const HEAD_TRACKING_KEY: &str = "controls.head_tracking";
/// `controls.head_tracking_gain`: the look per degree of head turn.
pub const HEAD_TRACKING_GAIN_KEY: &str = "controls.head_tracking_gain";

/// The descriptors of both settings.
#[must_use]
pub fn descriptors() -> Vec<D> {
    vec![
        D::new(
            HEAD_TRACKING_KEY,
            K::Bool,
            "false",
            C::Controls,
            50,
            "set-head-tracking",
            "set-head-tracking-desc",
        )
        .keywords(&["head", "trackir", "tracking", "view", "camera"]),
        D::new(
            HEAD_TRACKING_GAIN_KEY,
            K::Float {
                min: 0.0,
                max: f64::from(MAX_GAIN),
                step: 0.05,
            },
            "1.0",
            C::Controls,
            51,
            "set-head-tracking-gain",
            "set-head-tracking-gain-desc",
        )
        .keywords(&["head", "trackir", "gain", "sensitivity"]),
    ]
}

/// Copies the two settings into `HeadTrackSettings`. A missing or invalid gain is the default.
pub fn sync_head_track_settings(settings: Res<GameSettings>, mut head: ResMut<HeadTrackSettings>) {
    let enabled = settings
        .raw(HEAD_TRACKING_KEY)
        .is_some_and(|v| v.trim() == "true");
    let gain = settings
        .raw(HEAD_TRACKING_GAIN_KEY)
        .and_then(|v| v.trim().parse::<f32>().ok())
        .filter(|g| g.is_finite())
        .unwrap_or(DEFAULT_GAIN)
        .clamp(0.0, MAX_GAIN);
    let next = HeadTrackSettings { enabled, gain };
    if *head != next {
        tracing::debug!(enabled, gain, "head-tracking settings changed");
        *head = next;
    }
}
