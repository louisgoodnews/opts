//! In-world gizmo overlay for the scene trace (`TraceOverlay`).

use super::*;

/// The overlay of the developer panel: wire boxes coloured by decode quality around the camera.
#[derive(Resource, Debug, Clone, Default)]
pub struct TraceOverlay {
    /// Whether the overlay draws.
    pub enabled: bool,
    /// The record highlighted (the last `trace.pick`).
    pub picked: Option<u64>,
    /// Radius around the camera (m); 0 uses the default.
    pub radius: f32,
}

/// Default radius of the overlay (m).
const OVERLAY_RADIUS_M: f32 = 120.0;
/// Most boxes the overlay draws in one frame.
const OVERLAY_MAX_BOXES: usize = 1500;

/// Adds the overlay resource and system.
pub fn add_overlay(app: &mut App) {
    app.init_resource::<TraceOverlay>().add_systems(
        PostUpdate,
        draw_overlay.run_if(resource_exists::<bevy::gizmos::config::GizmoConfigStore>),
    );
}

fn colour(quality: &str, kind: &str) -> Color {
    match quality {
        "failed" => Color::srgb(1.0, 0.1, 0.1),
        "unreliable" => Color::srgb(1.0, 0.6, 0.0),
        "decoded" => Color::srgb(1.0, 0.9, 0.2),
        _ => match kind {
            st::kind::PARKED | st::kind::TRAFFIC | st::kind::VEHICLE => Color::srgb(0.3, 0.7, 1.0),
            st::kind::HUMAN => Color::srgb(0.8, 0.4, 1.0),
            st::kind::TREE => Color::srgb(0.2, 0.8, 0.3),
            _ => Color::srgb(0.6, 0.9, 0.6),
        },
    }
}

fn draw_overlay(
    overlay: Res<TraceOverlay>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut gizmos: Gizmos,
) {
    if !overlay.enabled || !st::enabled() {
        return;
    }
    let Ok(cam) = camera.single() else { return };
    let radius = if overlay.radius > 0.0 {
        overlay.radius
    } else {
        OVERLAY_RADIUS_M
    };
    let c = cam.translation();
    for r in st::overlay_snapshot([c.x, c.z], radius, OVERLAY_MAX_BOXES) {
        let (Some(w), Some(b)) = (r.world, r.bounds) else {
            continue;
        };
        let (min, max) = (Vec3::from(b.min), Vec3::from(b.max));
        let local = Transform::from_translation((min + max) * 0.5)
            .with_scale((max - min).max(Vec3::splat(0.05)));
        let world = Transform {
            translation: Vec3::from(w.t),
            rotation: Quat::from_xyzw(w.q[0], w.q[1], w.q[2], w.q[3]),
            scale: Vec3::from(w.s),
        };
        let m = world.to_matrix() * local.to_matrix();
        let picked = overlay.picked == Some(r.id);
        let col = if picked {
            Color::WHITE
        } else {
            colour(r.quality(), &r.kind)
        };
        gizmos.cube(Transform::from_matrix(m), col);
    }
}
