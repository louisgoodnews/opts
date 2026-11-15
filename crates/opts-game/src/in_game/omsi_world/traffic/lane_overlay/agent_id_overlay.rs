//! Agent ids of the traffic (Shift+F11): a short marker above each vehicle near the camera and
//! its id as a label at the vehicle's screen position. The labels are a fixed pool of UI text
//! nodes; nothing is spawned or drawn while the layer is off.

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_bevy::MainCamera;

use super::lanes::{bevy_point, session_xy};
use super::super::TrafficState;
use super::LaneOverlay;
use crate::in_game::omsi_world::WorldStats;
use crate::in_game::types::InGameEntity;

/// Vehicles further than this from the camera (m) get no marker or label.
const LABEL_RADIUS_M: f64 = 150.0;
/// Most markers and labels at once: the nearest vehicles.
const MAX_LABELS: usize = 40;
/// Height of the marker above the vehicle's origin (m).
const MARKER_M: f32 = 4.0;
const MARKER_COLOUR: Color = Color::srgb(1.0, 0.95, 0.6);

/// One label of the pool; the index is the rank of the vehicle it shows.
#[derive(Component)]
pub struct AgentLabel(pub usize);

/// The text of a vehicle's label.
#[must_use]
pub fn label_text(id: u64) -> String {
    format!("#{id}")
}

/// Up to `max` vehicles nearest to `centre` within `radius`, nearest first. Each vehicle is
/// `(id, position in the session frame, height)`.
#[must_use]
pub fn nearest_agents(
    agents: &[(u64, DVec2, f64)],
    centre: DVec2,
    radius: f64,
    max: usize,
) -> Vec<(u64, DVec2, f64)> {
    let mut near: Vec<(f64, (u64, DVec2, f64))> = agents
        .iter()
        .map(|&a| (a.1.distance(centre), a))
        .filter(|(d, _)| *d <= radius)
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    near.into_iter().take(max).map(|(_, a)| a).collect()
}

/// Every vehicle on the lanes as `(id, position, height)`.
fn agent_points(state: &TrafficState) -> Vec<(u64, DVec2, f64)> {
    let graph = state.world.graph();
    state
        .world
        .agents()
        .iter()
        .filter_map(|a| {
            let (p, _) = a.pose(graph)?;
            Some((a.id.0, p, a.height(graph).unwrap_or(0.0)))
        })
        .collect()
}

type LabelQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static AgentLabel,
        &'static mut Node,
        &'static mut Text,
        &'static mut Visibility,
    ),
>;

fn hide_labels(labels: &mut LabelQuery) {
    for (_, _, _, mut vis) in labels.iter_mut() {
        if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
}

fn spawn_pool(commands: &mut Commands) {
    for rank in 0..MAX_LABELS {
        commands.spawn((
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(MARKER_COLOUR),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            Visibility::Hidden,
            AgentLabel(rank),
            InGameEntity,
        ));
    }
}

/// Marks and labels the nearest vehicles while the layer is on; hides the labels otherwise.
pub fn update_agent_ids(
    mut commands: Commands,
    stats: Res<WorldStats>,
    overlay: Res<LaneOverlay>,
    traffic: Option<Res<TrafficState>>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut labels: LabelQuery,
    mut gizmos: Gizmos,
) {
    let shown = stats.overlay_visible && overlay.agent_ids;
    let (true, Some(traffic), Ok((cam, cam_gt))) = (shown, traffic, camera.single()) else {
        hide_labels(&mut labels);
        return;
    };
    let nearest = nearest_agents(
        &agent_points(&traffic),
        session_xy(cam_gt),
        LABEL_RADIUS_M,
        MAX_LABELS,
    );
    if labels.is_empty() {
        spawn_pool(&mut commands);
    }
    for &(_, p, h) in &nearest {
        let at = bevy_point(p, h);
        gizmos.line(at, at + Vec3::Y * MARKER_M, MARKER_COLOUR);
    }
    for (label, mut node, mut text, mut vis) in &mut labels {
        let Some(&(id, p, h)) = nearest.get(label.0) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let anchor = bevy_point(p, h) + Vec3::Y * MARKER_M;
        let Ok(screen) = cam.world_to_viewport(cam_gt, anchor) else {
            *vis = Visibility::Hidden;
            continue;
        };
        node.left = Val::Px(screen.x);
        node.top = Val::Px(screen.y);
        let s = label_text(id);
        if text.0 != s {
            text.0 = s;
        }
        *vis = Visibility::Visible;
    }
}

#[cfg(test)]
mod tests;
