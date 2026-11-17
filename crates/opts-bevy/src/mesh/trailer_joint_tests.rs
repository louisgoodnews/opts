//! Tests of the trailer-front joint remap: the trailer-end band, the remap of ring D, and the rest
//! pose with the new joint at identity. The Kajosoft bellow is tested from the game files in
//! `opts-game/tests/trailer_joint_bellow.rs`.

use bevy::math::{Mat4, Vec3};

use super::trailer_joint::{skin_point, SkinEntry};
use super::*;

const RING_D: u16 = 3;
const IDENTITY_JOINT: u16 = 4;
const TRAILER_JOINT: u16 = 5;

/// Four vertices: a head-end vertex (identity joint), two trailer-end vertices with ring D and one
/// trailer-end vertex without ring D (its ring-D entry is padding with weight 0).
fn small_table() -> Vec<SkinEntry> {
    vec![
        ([IDENTITY_JOINT, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
        ([RING_D, 2, 0, 0], [0.6, 0.4, 0.0, 0.0]),
        ([RING_D, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
        ([2, RING_D, 0, 0], [1.0, 0.0, 0.0, 0.0]),
    ]
}

fn small_bellow() -> [Vec3; 4] {
    [
        Vec3::new(0.0, 0.0, 2.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.05, 0.0),
        Vec3::new(0.0, 0.0, 0.031),
    ]
}

#[test]
fn band_is_the_lowest_slab_along_the_axis() {
    let p = small_bellow();
    assert_eq!(trailer_end_vertices(&p, Vec3::Z), vec![1, 2]);
    assert_eq!(trailer_end_vertices(&p, Vec3::Y), vec![0, 1, 3]);
}

#[test]
fn an_empty_mesh_has_no_trailer_end() {
    assert!(trailer_end_vertices(&[], Vec3::Z).is_empty());
}

#[test]
fn the_kajosoft_axis_is_forward_in_omsi_and_z_in_the_o3d_file() {
    let coupling = Vec3::new(0.0, -4.421, 0.4);
    let hitch = Vec3::new(0.0, 4.116, 0.4);
    let axis = trailer_end_axis(coupling, hitch).expect("two points");
    assert!(axis.abs_diff_eq(Vec3::Y, 1e-6), "{axis:?}");
    assert!(raw_dir_from_omsi(axis).abs_diff_eq(Vec3::Z, 1e-6));
    assert!(model_dir_from_omsi(axis).abs_diff_eq(-Vec3::Z, 1e-6));
}

#[test]
fn the_axis_needs_two_distinct_points() {
    assert!(trailer_end_axis(Vec3::ONE, Vec3::ONE).is_none());
}

#[test]
fn the_dominant_joint_is_the_one_carrying_most_of_the_band() {
    let table = small_table();
    // Vertices 1 and 2 carry ring D; vertex 1 also carries joint 2 (weight 0.4).
    assert_eq!(dominant_joint(table[1..3].iter()), Some(RING_D));
    assert_eq!(dominant_joint(std::iter::empty::<&SkinEntry>()), None);
}

#[test]
fn remap_changes_only_the_ring_d_entries_of_trailer_end_vertices() {
    let table = small_table();
    let (out, n) = remap_trailer_end(&table, &[1, 2, 3], RING_D, TRAILER_JOINT);
    assert_eq!(n, 2, "vertex 3 has no positive ring-D weight");
    assert_eq!(out[0], table[0], "head end is untouched");
    assert_eq!(out[1], ([TRAILER_JOINT, 2, 0, 0], [0.6, 0.4, 0.0, 0.0]));
    assert_eq!(out[2], ([TRAILER_JOINT, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]));
    assert_eq!(out[3], table[3], "ring D is only padding there");
}

#[test]
fn an_empty_trailer_end_set_is_a_no_op() {
    let table = small_table();
    let (out, n) = remap_trailer_end(&table, &[], RING_D, TRAILER_JOINT);
    assert_eq!(n, 0);
    assert_eq!(out, table);
}

#[test]
fn a_band_without_ring_d_remaps_nothing() {
    let table = vec![([IDENTITY_JOINT, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]); 3];
    let (out, n) = remap_trailer_end(&table, &[0, 1, 2], RING_D, TRAILER_JOINT);
    assert_eq!(n, 0);
    assert_eq!(out, table);
}

#[test]
fn rest_pose_with_identity_joints_equals_the_authored_positions() {
    let points = [
        Vec3::new(0.3, 0.1, 2.0),
        Vec3::new(-0.2, 0.5, 0.0),
        Vec3::new(0.7, 0.9, 0.01),
        Vec3::new(0.1, 0.4, 0.02),
    ];
    let table = small_table();
    let (out, _) = remap_trailer_end(&table, &[1, 2, 3], RING_D, TRAILER_JOINT);
    let mats = vec![Mat4::IDENTITY; usize::from(TRAILER_JOINT) + 1];
    for ((p, before), after) in points.iter().zip(&table).zip(&out) {
        let posed_before = skin_point(before, &mats, *p);
        let posed_after = skin_point(after, &mats, *p);
        assert_eq!(
            posed_after, posed_before,
            "bit-identical to the current rest pose"
        );
        assert!(
            posed_after.abs_diff_eq(*p, 1e-6),
            "{posed_after:?} != {p:?}"
        );
    }
}

#[test]
fn moving_the_trailer_joint_moves_only_the_trailer_end_vertices() {
    let table = small_table();
    let (out, _) = remap_trailer_end(&table, &[1, 2, 3], RING_D, TRAILER_JOINT);
    let mut mats = vec![Mat4::IDENTITY; usize::from(TRAILER_JOINT) + 1];
    mats[usize::from(TRAILER_JOINT)] = Mat4::from_translation(Vec3::X);
    let moved: Vec<f32> = out
        .iter()
        .map(|e| skin_point(e, &mats, Vec3::ZERO).x)
        .collect();
    assert_eq!(moved, vec![0.0, 0.6, 1.0, 0.0]);
}
