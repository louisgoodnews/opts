//! Queries by file, stage, tile and id, and the ray pick.

use super::*;

#[test]
fn query_by_file_stage_and_tile_and_get() {
    let _g = guard();
    set_directory(scratch("query"));
    set_mode(Mode::Full);
    begin_session("Q", Path::new("/omsi"));
    let (mut r, w) = rec(
        kind::SCENERY,
        (1, 2),
        "Sceneryobjects/h/house.sco",
        [0.0; 3],
    );
    r.meshes.push(MeshNote {
        file: "Sceneryobjects/h/model/house.o3d".into(),
        stage: "decode-cache".into(),
        ..MeshNote::default()
    });
    let id = record(
        &Probe {
            kind: kind::SCENERY,
            tile: Some((1, 2)),
            file: "house.sco",
            world: w,
        },
        || r,
    )
    .unwrap();
    put(kind::SCENERY, (1, 2), "other.sco", [0.0; 3]).unwrap();
    let by_mesh = query(&Query {
        file: Some("model/house.o3d".into()),
        ..Query::default()
    });
    assert_eq!(by_mesh.len(), 1);
    let by_stage = query(&Query {
        stage: Some("decode-cache".into()),
        tile: Some((1, 2)),
        ..Query::default()
    });
    assert_eq!(by_stage.len(), 1);
    assert_eq!(get(id).unwrap().file, "Sceneryobjects/h/house.sco");
    begin_tile((1, 2));
    assert!(
        query(&Query::default()).is_empty(),
        "a rebuilt tile supersedes its records"
    );
    set_mode(Mode::Off);
}

#[test]
fn pick_returns_the_nearest_box_on_the_ray() {
    let (near, _) = rec(kind::SCENERY, (0, 0), "near", [0.0, 0.0, -10.0]);
    let (far, _) = rec(kind::SCENERY, (0, 0), "far", [0.0, 0.0, -30.0]);
    let (aside, _) = rec(kind::SCENERY, (0, 0), "aside", [20.0, 0.0, -10.0]);
    let mut recs = vec![near, far, aside];
    for (i, r) in recs.iter_mut().enumerate() {
        r.id = i as u64 + 1;
    }
    let hits = pick::pick(&recs, Vec3::ZERO, Vec3::NEG_Z);
    assert_eq!(hits.iter().map(|h| h.id).collect::<Vec<_>>(), vec![1, 2]);
    assert!((hits[0].distance - 9.0).abs() < 1e-4);
    // A rotated, scaled box.
    let mut r = recs[0].clone();
    let w = r.world.as_mut().unwrap();
    w.q = bevy::math::Quat::from_rotation_y(0.5).to_array();
    w.s = [2.0, 1.0, 1.0];
    assert!(pick::ray_hits(&r, Vec3::ZERO, Vec3::NEG_Z).is_some());
    assert!(pick::ray_hits(&r, Vec3::new(0.0, 5.0, 0.0), Vec3::NEG_Z).is_none());
}
