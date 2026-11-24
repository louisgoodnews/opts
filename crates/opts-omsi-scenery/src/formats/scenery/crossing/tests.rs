//! Tests of the crossing ground faces and base surfaces.

use super::*;
use crate::formats::o3d::Vertex;
use crate::formats::scenery::{ScoPath, ScoSplineHelper};

/// A 40 × 40 m base rising 2 m from z = −20 to z = +20 (x across).
fn tilted() -> BaseSurface {
    let p = |x: f32, z: f32| Vec3::new(x, 1.0 + z / 20.0, z);
    BaseSurface::from_triangles(vec![
        [p(-20.0, -20.0), p(20.0, -20.0), p(-20.0, 20.0)],
        [p(20.0, -20.0), p(20.0, 20.0), p(-20.0, 20.0)],
    ])
}

fn mesh(points: &[Vec3]) -> O3d {
    O3d {
        version: 3,
        header_flags: 0,
        vertices: points
            .iter()
            .map(|&position| Vertex {
                position,
                normal: Vec3::Y,
                uv: Vec2::ZERO,
                color: None,
            })
            .collect(),
        indices: vec![[0, 1, 2]],
        face_materials: vec![0],
        materials: Vec::new(),
        transform: None,
        encryption_key: None,
        exact_decoded: false,
        unknown_chunks: Vec::new(),
    }
}

#[test]
fn the_keyword_names_the_base_mesh() {
    let text = "[crossing_heightdeformation]\r\nKreuz_base.o3d\r\n\r\n[surface]\r\n";
    let sco = SceneryObject::parse(text.as_bytes()).expect("parses");
    assert_eq!(sco.crossing_deformation.as_deref(), Some("Kreuz_base.o3d"));
    assert!(sco.surface);
}

#[test]
fn the_base_surface_bends_vertices_and_is_zero_outside() {
    let base = tilted();
    assert!((base.height(0.0, 0.0) - 1.0).abs() < 1e-5);
    assert!((base.height(5.0, 10.0) - 1.5).abs() < 1e-5);
    assert_eq!(base.height(30.0, 0.0), 0.0);
    let mut m = mesh(&[
        Vec3::new(0.0, 0.1, -20.0),
        Vec3::new(0.0, 0.1, 20.0),
        Vec3::new(50.0, 0.1, 0.0),
    ]);
    base.bend_mesh(&mut m);
    let y: Vec<f32> = m.vertices.iter().map(|v| v.position.y).collect();
    assert!((y[0] - 0.1).abs() < 1e-5 && (y[1] - 2.1).abs() < 1e-5);
    assert!(
        (y[2] - 0.1).abs() < 1e-5,
        "outside the footprint: unchanged"
    );
}

#[test]
fn paths_and_arms_follow_the_base() {
    let base = tilted();
    let mut sco = SceneryObject::default();
    // A straight path north from (0, −10) for 20 m, 0.1 m up, flat as written.
    sco.paths.push(ScoPath {
        start: Vec3::new(0.0, -10.0, 0.1),
        heading_deg: 0.0,
        radius_m: 0.0,
        length_m: 20.0,
        slope_start: 0.0,
        slope_end: 0.0,
        path_type: 0,
        width_m: 3.0,
        direction: 0,
        extra: None,
        traffic_light_index: None,
        approach_distance_m: None,
        switch_position: None,
    });
    sco.spline_helpers.push(ScoSplineHelper {
        profile_file: "a.sli".into(),
        position: Vec3::new(0.0, 20.0, 0.0),
        rotation_deg: 180.0,
        extra: Vec::new(),
    });
    let bent = sco.bent(&base);
    let p = &bent.paths[0];
    // Height along the path: z0 + s·(g0 + (g1 − g0)·t/2).
    let z = |s: f32| p.start.z + s * (p.slope_start + (p.slope_end - p.slope_start) * s / 40.0);
    assert!((z(0.0) - 0.6).abs() < 1e-5);
    assert!((z(10.0) - 1.1).abs() < 1e-4);
    assert!((z(20.0) - 1.6).abs() < 1e-4);
    assert!((bent.spline_helpers[0].position.z - 2.0).abs() < 1e-5);
    // Without a base nothing changes.
    assert_eq!(sco.bent(&BaseSurface::default()), sco);
}

#[test]
fn ground_faces_are_level_faces_near_the_base() {
    let base = tilted();
    let floor = mesh(&[
        Vec3::new(0.0, 1.1, 0.0),
        Vec3::new(0.0, 1.1, 2.0),
        Vec3::new(2.0, 1.1, 0.0),
    ]);
    let wall = mesh(&[
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 2.0, 0.0),
        Vec3::new(2.0, 0.0, 0.0),
    ]);
    let roof = mesh(&[
        Vec3::new(0.0, 9.0, 0.0),
        Vec3::new(0.0, 9.0, 2.0),
        Vec3::new(2.0, 9.0, 0.0),
    ]);
    let faces = crossing_ground_faces(&[floor, wall, roof], Some(&base));
    assert_eq!(faces.len(), 1);
}
