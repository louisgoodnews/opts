//! The HUD's responsive layout: one decision per frame from the size of the layout area (the
//! window divided by the UI scale), so the bottom-left cluster, the bottom-right mini map, the
//! camera label and the toasts never overlap from 1024 x 640 up to 2560 x 1440 at a UI scale of
//! 75 to 200 per cent.
//!
//! A *compact* layout (a narrow or low layout area) gives up spacing and the long hint line, not
//! information: smaller paddings, narrower readouts, the short hint, a smaller map.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::parts::{ClusterBox, HudRoot, HudSlot, MapBox, ReadoutBox};
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

/// Applies the layout to the HUD's nodes (cheap: only differing values are written).
#[allow(clippy::type_complexity)]
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

/// Gap between the mini map (or the cluster) and the toast stack above it (layout px).
pub const TOAST_GAP: f32 = 8.0;
/// A toast is at least this wide (`opts_gui` toast), and never nearer to the right edge than
/// [`opts_gui::components::toast_stack::MARGIN`].
const TOAST_MIN_WIDTH: f32 = 280.0;

/// Raises the toast stack above the HUD at the bottom: toasts live at the bottom right too, and
/// must not cover the mini map. When the widest live toast would also reach into the cluster at the
/// left (a narrow layout) the stack clears the cluster as well. Without a visible map the inset is
/// `0` (the cluster alone is clear of the toasts when it stays left of them).
pub fn toast_inset(
    layout: Res<HudLayout>,
    maps: Query<(&ComputedNode, &Node, &Visibility), With<MapBox>>,
    clusters: Query<(&HudSlot, &ComputedNode, &UiGlobalTransform, &Node)>,
    toasts: Query<&ComputedNode, With<opts_gui::components::toast::M3Toast>>,
    mut inset: ResMut<opts_gui::components::toast_stack::ToastInset>,
) {
    use opts_gui::components::toast_stack::MARGIN;
    let k = |c: &ComputedNode| c.inverse_scale_factor();
    let map_height = maps
        .iter()
        .find(|(_, n, v)| n.display != Display::None && **v != Visibility::Hidden)
        .map_or(0.0, |(c, _, _)| c.size().y * k(c));
    // The cluster: its right edge and height.
    let cluster = clusters
        .iter()
        .find(|(s, _, _, n)| s.0 == Slot::ClusterBottomLeft && n.display != Display::None)
        .map(|(_, c, t, _)| {
            let right = (t.translation.x + c.size().x / 2.0) * k(c);
            (right, c.size().y * k(c))
        });
    let widest = toasts
        .iter()
        .map(|c| c.size().x * k(c))
        .fold(TOAST_MIN_WIDTH, f32::max);
    let toast_left = layout.width - MARGIN - widest;
    let mut height = map_height;
    // While the editor's panel is open the toasts stay at the bottom edge, below the panel.
    if let (Some((right, cluster_height)), false) = (cluster, layout.editor) {
        if right + TOAST_GAP > toast_left {
            height = height.max(cluster_height);
        }
    }
    let want = if height > 1.0 {
        (layout.margin + height + TOAST_GAP - MARGIN).max(0.0)
    } else {
        0.0
    };
    if (inset.0 - want).abs() > 0.5 {
        inset.0 = want;
    }
}
