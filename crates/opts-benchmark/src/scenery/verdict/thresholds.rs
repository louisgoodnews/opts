//! Decision thresholds: the numbers the verdict rules compare against (recorded in `run.json`).

use serde::{Deserialize, Serialize};

/// Decision thresholds (recorded in `run.json`; calibrated values in the spec, Q-SB-02).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Thresholds {
    /// Fewer rendered pixels than this counts as an empty render.
    pub empty_px: usize,
    /// Expected pixels needed before an empty render is a failure.
    pub min_expected_px: usize,
    /// `F-GEOM-OUTSIDE` below this precision.
    pub precision: f32,
    /// `F-GEOM-OUTSIDE` above this bounding-box overshoot.
    pub bbox_overshoot: f32,
    /// `F-XFORM-ROTATION`: identity IoU below this …
    pub rotation_identity_iou: f32,
    /// … and an alternative orientation at or above this.
    pub rotation_best_iou: f32,
    /// `F-XFORM-OFFSET` above this centroid offset (both views).
    pub centroid_offset: f32,
    /// `F-XFORM-SCALE` outside `[1 / scale, scale]` (both views).
    pub scale: f32,
    /// `F-GEOM-PARTIAL` below this recall.
    pub recall: f32,
    /// Alpha-tested tree billboards: below this recall the verdict is `unknown` (the coverage
    /// of an alpha texture is not predictable from the mesh). Other mostly alpha-tested
    /// objects are `unknown` below `recall`.
    pub recall_alpha: f32,
    /// An object is mostly alpha-tested above this alpha share.
    pub alpha_share: f32,
    /// `F-TEX-MISSING` above this sentinel share.
    pub sentinel_share: f32,
    /// `F-TEXT-NOT-SHOWN` below this share of changed text pixels (in both close-ups).
    pub text_changed_share: f32,
    /// A text pixel counts as changed when a channel differs by at least this much (0..255).
    pub text_delta: u8,
    /// The text check needs at least this many text pixels in a view.
    pub text_min_px: usize,
    /// Script-effect check thresholds.
    #[serde(default)]
    pub script: super::super::script_check::ScriptThresholds,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            empty_px: 20,
            min_expected_px: 200,
            precision: 0.90,
            bbox_overshoot: 0.25,
            rotation_identity_iou: 0.5,
            rotation_best_iou: 0.8,
            centroid_offset: 0.10,
            scale: 1.25,
            recall: 0.6,
            recall_alpha: 0.02,
            alpha_share: 0.5,
            sentinel_share: 0.02,
            text_changed_share: 0.002,
            text_delta: 16,
            text_min_px: 20,
            script: super::super::script_check::ScriptThresholds::default(),
        }
    }
}
