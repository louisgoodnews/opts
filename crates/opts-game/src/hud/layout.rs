//! The HUD's responsive layout: one decision per frame from the size of the layout area (the
//! window divided by the UI scale), so the bottom-left cluster, the bottom-right mini map, the
//! camera label and the toasts never overlap from 1024 x 640 up to 2560 x 1440 at a UI scale of
//! 75 to 200 per cent.
//!
//! A *compact* layout (a narrow or low layout area) gives up spacing and the long hint line, not
//! information: smaller paddings, narrower readouts, the short hint, a smaller map.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::parts::{ClusterBox, HudRoot, MapBox, ReadoutBox};

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
            map_side: (height * MAP_HEIGHT_SHARE).clamp(MAP_SIDE_RANGE.0, MAP_SIDE_RANGE.1),
            margin: if compact { 12.0 } else { 20.0 },
        }
    }
}

impl HudLayout {
    /// The most the cluster may be wide: the area minus the map and the gaps.
    #[must_use]
    pub fn cluster_max_width(&self) -> f32 {
        (self.width - self.map_side - 2.0 * self.margin - 12.0).max(120.0)
    }
}

/// Measures the layout area and keeps [`HudLayout`] current.
pub fn measure(
    window: Query<&Window, With<PrimaryWindow>>,
    scale: Option<Res<UiScale>>,
    mut layout: ResMut<HudLayout>,
) {
    let Ok(window) = window.single() else { return };
    let ui = scale.map_or(1.0, |s| s.0).max(0.1);
    let (w, h) = (window.width() / ui, window.height() / ui);
    if w < 1.0 || h < 1.0 {
        return;
    }
    let next = HudLayout::for_area(w, h);
    layout.set_if_neq(next);
}

/// Applies the layout to the HUD's nodes (cheap: only differing values are written).
pub fn apply(
    layout: Res<HudLayout>,
    mut roots: Query<&mut Node, (With<HudRoot>, Without<MapBox>, Without<ReadoutBox>)>,
    mut maps: Query<&mut Node, (With<MapBox>, Without<HudRoot>, Without<ReadoutBox>)>,
    mut readouts: Query<(&ReadoutBox, &mut Node), (Without<HudRoot>, Without<MapBox>)>,
    mut clusters: Query<
        &mut Node,
        (
            With<ClusterBox>,
            Without<HudRoot>,
            Without<MapBox>,
            Without<ReadoutBox>,
        ),
    >,
) {
    for mut node in &mut clusters {
        let want = Val::Px(layout.cluster_max_width());
        if node.max_width != want {
            node.max_width = want;
        }
    }
    for mut node in &mut roots {
        let want = UiRect::all(Val::Px(layout.margin));
        if node.padding != want {
            node.padding = want;
        }
    }
    for mut node in &mut maps {
        let want = Val::Px(layout.map_side);
        if node.width != want {
            node.width = want;
        }
    }
    for (box_, mut node) in &mut readouts {
        let want = Val::Px(box_.width(layout.compact));
        if node.min_width != want {
            node.min_width = want;
        }
    }
}
