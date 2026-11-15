//! Tuning of the mirror cameras: sizes, ranges, the user adjustment and the `MirrorSettings`.

use bevy::prelude::*;

/// Most mirror groups (cameras) that exist; at most two render at a time.
pub const MAX_CAMERAS: usize = 8;
/// Real seconds between two scans of the player vehicle for mirror panes.
pub const SCAN_INTERVAL_S: f64 = 1.0;
/// Distance of the camera in front of the pane (m).
pub const MIRROR_OFFSET_M: f32 = 0.05;
/// Smallest and largest vertical field of view of a mirror picture (degrees).
pub const MIRROR_FOV_RANGE_DEG: (f32, f32) = (20.0, 70.0);
/// Range of the projection aspect (width / height) of a mirror picture.
pub const MIRROR_ASPECT_RANGE: (f32, f32) = (0.4, 3.0);
/// A pane counts as in front of the camera when the cosine of the angle between the view
/// direction and the direction to the pane is above this (about 78 degrees).
pub const MIRROR_VIEW_COS: f32 = 0.2;
/// Draw order of the mirror cameras: before the main camera.
pub const MIRROR_ORDER: isize = -10;

/// A user adjustment on top of the `.bus` mirror cameras (degrees), for a mirror whose tags do not
/// suit the user's seat or taste. `OPTS_MIRROR_ADJUST=yaw,pitch,fov` (any number of the three).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MirrorAdjust {
    /// Added to the tag's yaw (positive = further counter-clockwise seen from above).
    pub yaw_deg: f32,
    /// Added to the tag's pitch (positive = up).
    pub pitch_deg: f32,
    /// Added to the tag's field of view.
    pub fov_deg: f32,
}

/// Where the mirror pictures go and how they are laid on the pane.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct MirrorSettings {
    /// Texture size in pixels.
    pub size: UVec2,
    /// Mirror the picture left-right on the pane. Off by default: the panes of the inspected
    /// models carry UVs that already run right to left as the driver sees them (the texture is
    /// stored mirror-reversed), so the plain camera picture is the mirror image.
    pub flip_u: bool,
    /// Turn the picture upside down on the pane (a dev aid, off).
    pub flip_v: bool,
    /// Real seconds between two scans for new panes ([`SCAN_INTERVAL_S`]).
    pub scan_interval_s: f64,
    /// Drive the cameras from the vehicle's `[add_camera_reflexion]` tags (position, direction,
    /// field of view, square picture) when it has one for the pane; `false` keeps the geometric
    /// viewer-ray reflection for every pane. `OPTS_MIRROR_TAGS=0` switches it off.
    pub use_tags: bool,
    /// Adjustment of the tag cameras.
    pub adjust: MirrorAdjust,
}

impl Default for MirrorSettings {
    fn default() -> Self {
        Self {
            size: UVec2::new(256, 128),
            flip_u: false,
            flip_v: false,
            scan_interval_s: SCAN_INTERVAL_S,
            use_tags: true,
            adjust: MirrorAdjust::default(),
        }
    }
}

impl MirrorSettings {
    /// `OPTS_MIRROR_FLIP` (`u`, `v`, `uv` or `none`) and `OPTS_MIRROR_SIZE` (`256x128`) on top of
    /// the defaults: developer aids for checking the picture's orientation and cost.
    #[must_use]
    pub fn from_env() -> Self {
        let mut s = Self::default();
        if let Ok(f) = std::env::var("OPTS_MIRROR_FLIP") {
            s.flip_u = f.contains('u');
            s.flip_v = f.contains('v');
        }
        if let Ok(v) = std::env::var("OPTS_MIRROR_TAGS") {
            s.use_tags = !matches!(v.trim(), "0" | "off" | "false" | "no");
        }
        if let Ok(v) = std::env::var("OPTS_MIRROR_ADJUST") {
            let mut it = v.split(',').map(|n| n.trim().parse::<f32>().unwrap_or(0.0));
            s.adjust = MirrorAdjust {
                yaw_deg: it.next().unwrap_or(0.0),
                pitch_deg: it.next().unwrap_or(0.0),
                fov_deg: it.next().unwrap_or(0.0),
            };
        }
        if let Ok(v) = std::env::var("OPTS_MIRROR_SIZE") {
            if let Some((w, h)) = v.split_once('x') {
                if let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.parse::<u32>()) {
                    s.size = UVec2::new(w.clamp(32, 1024), h.clamp(16, 1024));
                }
            }
        }
        s
    }
}
