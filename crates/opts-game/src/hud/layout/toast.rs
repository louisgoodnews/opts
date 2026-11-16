//! The toast inset: raises the toast stack above the HUD's bottom slots.

use super::*;

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
