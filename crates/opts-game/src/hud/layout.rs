//! The HUD's responsive layout: one decision per frame from the size of the layout area (the
//! window divided by the UI scale), so the bottom-left cluster, the bottom-right mini map, the
//! camera label and the toasts never overlap from 1024 x 640 up to 2560 x 1440 at a UI scale of
//! 75 to 200 per cent.
//!
//! A *compact* layout (a narrow or low layout area) gives up spacing and the long hint line, not
//! information: smaller paddings, narrower readouts, the short hint, a smaller map.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::parts::{CameraLabelRow, ClusterBox, HudRoot, HudSlot, MapBox, ReadoutBox};
use super::widget::Slot;

/// The layout area is narrower than this (logical px): compact.
pub const COMPACT_BELOW_WIDTH: f32 = 760.0;
/// The layout area is lower than this: compact.
pub const COMPACT_BELOW_HEIGHT: f32 = 420.0;
/// Smallest and largest side of the map square.
pub const MAP_SIDE_RANGE: (f32, f32) = (112.0, 224.0);
/// The share of the layout height the map square takes.
pub const MAP_HEIGHT_SHARE: f32 = 0.26;

/// What the HUD layout decided.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct HudLayout {
    /// Compact spacing and the short hint.
    pub compact: bool,
    /// Side of the mini map square (layout px).
    pub map_side: f32,
    /// Width of the layout area.
    pub width: f32,
    /// The hint row has room (a low layout area drops it: the cluster would reach the camera
    /// label, and the hints are on the Help page).
    pub hints: bool,
    /// The viewpoint editor is open: its panel takes the right edge.
    pub editor: bool,
    /// Padding of the HUD root (distance from the window edges).
    pub margin: f32,
}

impl Default for HudLayout {
    fn default() -> Self {
        Self::for_area(1280.0, 720.0)
    }
}

impl HudLayout {
    /// The layout for a layout area of `width` x `height` (window size divided by UI scale).
    #[must_use]
    pub fn for_area(width: f32, height: f32) -> Self {
        let compact = width < COMPACT_BELOW_WIDTH || height < COMPACT_BELOW_HEIGHT;
        Self {
            compact,
            width,
            hints: height >= COMPACT_BELOW_HEIGHT,
            editor: false,
            map_side: (height * MAP_HEIGHT_SHARE).clamp(MAP_SIDE_RANGE.0, MAP_SIDE_RANGE.1),
            margin: if compact { 12.0 } else { 20.0 },
        }
    }
}

impl HudLayout {
    /// The most the cluster may be wide: the area minus the map and the gaps.
    #[must_use]
    pub fn cluster_max_width(&self) -> f32 {
        let right = if self.editor {
            // The editor's panel is at the right edge (the map steps aside for it).
            VIEWPOINT_PANEL_WIDTH + 12.0
        } else {
            self.map_side + 12.0
        };
        let room = (self.width - right - 2.0 * self.margin).max(120.0);
        // With the editor open the cluster is only as wide as its instrument row, so that the
        // toasts at the bottom edge stay clear of it.
        if self.editor {
            room.min(EDITOR_CLUSTER_WIDTH)
        } else {
            room
        }
    }
}

/// The widest the cluster is while the viewpoint editor is open (layout px).
pub const EDITOR_CLUSTER_WIDTH: f32 = 300.0;

/// Width of the viewpoint editor's panel (layout px), see `hud::viewpoint`.
pub const VIEWPOINT_PANEL_WIDTH: f32 = 380.0;

/// Measures the layout area and keeps [`HudLayout`] current.
pub fn measure(
    window: Query<&Window, With<PrimaryWindow>>,
    scale: Option<Res<UiScale>>,
    viewpoint: Option<Res<opts_bevy::ViewpointState>>,
    mut layout: ResMut<HudLayout>,
) {
    let Ok(window) = window.single() else { return };
    let ui = scale.map_or(1.0, |s| s.0).max(0.1);
    let (w, h) = (window.width() / ui, window.height() / ui);
    if w < 1.0 || h < 1.0 {
        return;
    }
    let mut next = HudLayout::for_area(w, h);
    next.editor = viewpoint.is_some_and(|v| v.editing);
    next.hints = next.hints && !next.editor;
    layout.set_if_neq(next);
}

/// The camera label never sits higher than this (layout px from the top).
pub const CAMERA_LABEL_MIN_TOP: f32 = 84.0;
/// Gap between the status bar and the camera label below it (layout px).
const CAMERA_LABEL_GAP: f32 = 8.0;

/// Puts the camera label below the status bar: the bar grows with the text size, so a fixed
/// offset would let the two overlap at larger sizes.
pub fn place_camera_label(
    layout: Res<HudLayout>,
    status: Query<(&HudSlot, &ComputedNode)>,
    mut rows: Query<&mut Node, With<CameraLabelRow>>,
) {
    let bar = status
        .iter()
        .find(|(s, _)| s.0 == Slot::StatusBar)
        .map_or(0.0, |(_, c)| c.size().y * c.inverse_scale_factor());
    let want = Val::Px((layout.margin + bar + CAMERA_LABEL_GAP).max(CAMERA_LABEL_MIN_TOP));
    for mut node in &mut rows {
        if node.top != want {
            node.top = want;
        }
    }
}

mod apply;
mod toast;
pub use apply::apply;
pub use toast::{toast_inset, TOAST_GAP};

