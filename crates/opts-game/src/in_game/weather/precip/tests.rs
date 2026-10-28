//! Unit tests for `super` (`precip.rs`).

use super::*;
use opts_bevy::WeatherCatalogue;

fn rain() -> PrecipPlan {
    precip_plan(
        &WeatherCatalogue::default().resolve("Rainy Afternoon"),
        3000,
    )
    .unwrap()
}

#[test]
fn seeds_are_stable_and_spread() {
    assert_eq!(seed_of(7), seed_of(7));
    assert_ne!(seed_of(7), seed_of(8));
    let mean: f32 = (0..2000).map(|i| seed_of(i)[1]).sum::<f32>() / 2000.0;
    assert!((mean - 0.5).abs() < 0.05, "{mean}");
    assert!((0..2000).all(|i| seed_of(i).iter().all(|v| (0.0..1.0).contains(v))));
}

#[test]
fn particles_stay_in_the_box_around_the_camera_and_fall() {
    let plan = rain();
    let cam = Vec3::new(1234.5, 40.0, -987.0);
    for i in 0..500 {
        for t in [0.0, 3.3, 1.0e5] {
            let p = particle_position(seed_of(i), t, &plan, cam);
            assert!((p.x - cam.x).abs() <= BOX_XZ / 2.0 + 1e-3, "{p}");
            assert!((p.z - cam.z).abs() <= BOX_XZ / 2.0 + 1e-3, "{p}");
            assert!(
                p.y >= cam.y - BELOW_CAMERA - 1e-3 && p.y <= cam.y + BOX_Y,
                "{p}"
            );
        }
    }
    // Without wrapping a drop is 9 m lower after a second.
    let seed = [0.5, 0.9, 0.5];
    let a = particle_position(seed, 10.0, &plan, cam);
    let b = particle_position(seed, 10.1, &plan, cam);
    assert!(
        ((a.y - b.y) - plan.fall_speed * 0.1).abs() < 1e-3,
        "{a} {b}"
    );
    // The wind carries it sideways by the plan's drift.
    assert!(((b.x - a.x) - plan.drift.x * 0.1).abs() < 1e-3);
}

#[test]
fn particles_do_not_move_with_the_camera() {
    let plan = rain();
    let seed = seed_of(42);
    let a = particle_position(seed, 5.0, &plan, Vec3::new(0.0, 10.0, 0.0));
    // The camera moves 3 m east: the same drop is still in the window and has not moved.
    let b = particle_position(seed, 5.0, &plan, Vec3::new(3.0, 10.0, 0.0));
    let same_place = (a - b).length() < 1e-3;
    let wrapped = (a.x - b.x).abs() > BOX_XZ - 4.0;
    assert!(same_place || wrapped, "{a} {b}");
}

#[test]
fn a_streak_lies_along_the_fall_direction_and_faces_the_camera() {
    let plan = rain();
    let cam = Vec3::ZERO;
    let p = Vec3::new(0.0, 0.0, -10.0);
    let [s0, s1, e1, e0] = rain_corners(p, &plan, cam, Vec3::X);
    let axis = Vec3::new(plan.drift.x, -plan.fall_speed, plan.drift.y).normalize();
    assert!((e0 - s0).normalize().dot(axis) > 0.999);
    assert!(((e0 - s0).length() - plan.size.y).abs() < 1e-4);
    assert!(((s1 - s0).length() - plan.size.x).abs() < 1e-4);
    // The strip is perpendicular to the view direction.
    assert!((s1 - s0).normalize().dot((p - cam).normalize()).abs() < 1e-3);
    assert!(e1.is_finite());
}

#[test]
fn flakes_face_the_camera() {
    let mut plan = rain();
    plan.kind = PrecipKind::Snow;
    plan.size = Vec2::splat(0.1);
    let c = flake_corners(Vec3::new(1.0, 2.0, 3.0), &plan, Vec3::X, Vec3::Y);
    assert!((c[2] - c[0]).length() > 0.13);
    assert!(((c[1] - c[0]) - Vec3::new(0.1, 0.0, 0.0)).length() < 1e-5);
}

#[test]
fn the_quad_mesh_is_consistent() {
    let (mesh, indices) = quad_mesh(10);
    assert_eq!(mesh.count_vertices(), 40);
    assert_eq!(indices.len(), 60);
    assert!(indices.iter().all(|i| *i < 40));
}
