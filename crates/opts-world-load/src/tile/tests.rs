//! Unit tests of `tile.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::attached::attachment_transform;
use super::*;

/// The shape of `Vehicles/FW_AstraG/model/Astra_Parked.cfg`: the body in the close-up level,
/// `[LOD] 0` holding a bare file name without `[mesh]`.
const PARKED: &str = "[LOD]\r\n0.025\r\n\r\n[mesh]\r\nbody.o3d\r\n\r\n[mesh]\r\nwheels.o3d\r\n\r\n[LOD]\r\n0\r\n\r\n..\\..\\other\\model\\far.o3d\r\n";

#[test]
fn a_parked_model_without_meshes_at_lod_0_draws_its_close_up_level() {
    let model = ModelConfig::parse_bytes(PARKED.as_bytes()).expect("parses");
    let order = lod_preference(&model.lods);
    assert_eq!(order.len(), 1);
    assert!((order[0].threshold - 0.025).abs() < 1e-6);
    assert_eq!(order[0].meshes.len(), 2);
}

#[test]
fn an_attached_object_sits_on_the_turned_parent_and_follows_the_point_rotation() {
    use opts_core::formats::scenery::sco::ScoAttachment;
    // Parent facing east (heading 90, clockwise), point 1 m forward and 2 m up.
    let parent = Transform::from_rotation(omsi_euler_to_bevy_quat(-90.0, 0.0, 0.0));
    let point = ScoAttachment {
        translation: Vec3::new(0.0, 1.0, 2.0),
        rotation_deg: Vec3::ZERO,
    };
    let t = attachment_transform(&parent, &point, 0.0, 0.0, 0.0);
    assert!(
        (t.translation - Vec3::new(1.0, 2.0, 0.0)).length() < 1e-5,
        "{t:?}"
    );
    // The child faces where the parent faces: east (Bevy +x).
    let forward = t.rotation * Vec3::NEG_Z;
    assert!((forward - Vec3::X).length() < 1e-5, "{forward}");

    // `attach_rot_z 90` turns the child counter-clockwise seen from above: north -> west.
    let point = ScoAttachment {
        translation: Vec3::ZERO,
        rotation_deg: Vec3::new(0.0, 0.0, 90.0),
    };
    let t = attachment_transform(&Transform::IDENTITY, &point, 0.0, 0.0, 0.0);
    let forward = t.rotation * Vec3::NEG_Z;
    assert!((forward - Vec3::NEG_X).length() < 1e-5, "{forward}");
    // The child's own heading is clockwise like an [object]: 90 -> east.
    let t = attachment_transform(
        &Transform::IDENTITY,
        &ScoAttachment::default(),
        90.0,
        0.0,
        0.0,
    );
    let forward = t.rotation * Vec3::NEG_Z;
    assert!((forward - Vec3::X).length() < 1e-5, "{forward}");
}

/// The shape of `TH_A4_B6_Avant_parked.cfg`: full model at `0.07`, coarser ones below.
#[test]
fn the_most_detailed_level_comes_first() {
    let cfg = "[LOD]\r\n0.07\r\n\r\n[mesh]\r\nbody.o3d\r\n\r\n[mesh]\r\nwheels.o3d\r\n\r\n[LOD]\r\n0.04\r\n\r\n[mesh]\r\nbody_lod.o3d\r\n\r\n[LOD]\r\n0\r\n\r\n[mesh]\r\nbody_lod_2.o3d\r\n";
    let model = ModelConfig::parse_bytes(cfg.as_bytes()).expect("parses");
    let order: Vec<f32> = lod_preference(&model.lods)
        .iter()
        .map(|l| l.threshold)
        .collect();
    assert_eq!(order, vec![0.07, 0.04, 0.0]);
}
