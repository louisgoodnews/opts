//! Road and underside faces wind so that top faces point up.

use super::*;

#[test]
fn top_faces_of_a_road_face_up() {
    for inst in layouts() {
        let mesh = ribbon(vec![pt(-4.0, 0.1), pt(0.0, 0.1), pt(4.0, 0.1)], &inst);
        let normals = triangle_normals(&mesh);
        assert!(!normals.is_empty());
        for n in normals {
            assert!(
                n.y > 0.9,
                "top face points {n:?} for heading {} radius {} slope {} mirrored {}",
                inst.heading_deg,
                inst.radius_m,
                inst.gradient_start,
                inst.mirrored
            );
        }
    }
}

#[test]
fn bottom_faces_do_not_face_up() {
    for inst in layouts() {
        // The same carriageway listed right to left is the underside.
        let mesh = ribbon(vec![pt(4.0, 0.1), pt(0.0, 0.1), pt(-4.0, 0.1)], &inst);
        for n in triangle_normals(&mesh) {
            assert!(n.y < -0.9, "underside points {n:?}");
        }
    }
}

#[test]
fn curb_walls_face_away_from_the_road() {
    // Left-to-right profile with a raised right curb: carriageway, riser, curb top.
    let inst = instance(0.0, 0.0, 0.0, false);
    let mesh = ribbon(
        vec![pt(-4.0, 0.1), pt(4.0, 0.1), pt(4.0, 0.25), pt(4.15, 0.25)],
        &inst,
    );
    let normals = triangle_normals(&mesh);
    assert!(normals.iter().any(|n| n.x < -0.9), "riser faces the road");
    assert!(normals
        .iter()
        .filter(|n| n.y.abs() > 0.9)
        .all(|n| n.y > 0.0));
}

#[test]
fn vertex_normals_of_a_road_point_up() {
    let inst = instance(123.0, 70.0, 0.02, true);
    let mesh = ribbon(vec![pt(-4.0, 0.1), pt(4.0, 0.1)], &inst);
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .and_then(|a| a.as_float3())
        .expect("normals");
    assert!(normals.iter().all(|n| n[1] > 0.9));
}
