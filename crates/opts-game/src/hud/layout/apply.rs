//! Applying the layout to the HUD's nodes: the cluster width, the root padding, the map side and the readouts.

use super::*;

type RootNodes<'w, 's> =
    Query<'w, 's, &'static mut Node, (With<HudRoot>, Without<MapBox>, Without<ReadoutBox>)>;
type MapNodes<'w, 's> =
    Query<'w, 's, &'static mut Node, (With<MapBox>, Without<HudRoot>, Without<ReadoutBox>)>;
type ReadoutNodes<'w, 's> =
    Query<'w, 's, (&'static ReadoutBox, &'static mut Node), (Without<HudRoot>, Without<MapBox>)>;
type ClusterNodes<'w, 's> = Query<
    'w,
    's,
    &'static mut Node,
    (
        With<ClusterBox>,
        Without<HudRoot>,
        Without<MapBox>,
        Without<ReadoutBox>,
    ),
>;

/// Applies the layout to the HUD's nodes (cheap: only differing values are written).
pub fn apply(
    layout: Res<HudLayout>,
    mut roots: RootNodes,
    mut maps: MapNodes,
    mut readouts: ReadoutNodes,
    mut clusters: ClusterNodes,
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
