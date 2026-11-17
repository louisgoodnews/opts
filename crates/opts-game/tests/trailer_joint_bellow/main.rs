//! The trailer-front joint on the Kajosoft Citybus 530 Facelift bellow (`przegub_zew.o3d`): the
//! trailer-end axis is OMSI `+y`, the band is 75 vertices all on ring D, and the rest pose is
//! exact. The rigid pose and the head end are in `rigid.rs`. Skipped without `OMSI_PATH`.

use std::path::PathBuf;

use bevy::math::{Mat4, Vec3};
use opts_bevy::mesh::{
    model_dir_from_omsi, raw_dir_from_omsi, remap_trailer_end, skin_table_slots, trailer_end_axis,
    trailer_end_vertices, SkinEntry,
};

mod rigid;
mod seam;

const RING_D: u16 = 3;
const TRAILER_JOINT: u16 = 5;
const COUPLING: Vec3 = Vec3::new(0.0, -4.421, 0.4);
const HITCH: Vec3 = Vec3::new(0.0, 4.116, 0.4);

/// The bellow in both frames: the Bevy model space of its meshes and OMSI.
struct Bellow {
    model: Vec<Vec3>,
    omsi: Vec<Vec3>,
    table: Vec<SkinEntry>,
    band: Vec<usize>,
}

fn load() -> Option<Bellow> {
    let root = PathBuf::from(std::env::var_os("OMSI_PATH")?);
    let path = root.join("Vehicles/Citybus 530 Facelift by Kajosoft/Model/C1_G/przegub_zew.o3d");
    let bytes = std::fs::read(path).ok()?;
    let o3d = opts_core::formats::o3d::parse(&bytes).expect("przegub_zew.o3d");
    let raw: Vec<Vec3> = o3d.vertices.iter().map(|v| v.position).collect();
    // The o3d file is in its own axes: its height is `y`, its length `z`. The meshes are in the
    // Bevy model space `(x, y, -z)`, and OMSI is `(x, z, y)` of the file.
    let model: Vec<Vec3> = raw.iter().map(|p| Vec3::new(p.x, p.y, -p.z)).collect();
    let omsi: Vec<Vec3> = raw.iter().map(|p| Vec3::new(p.x, p.z, p.y)).collect();
    let slots = [
        ("Gelenk_A", 0),
        ("Gelenk_B", 1),
        ("Gelenk_C", 2),
        ("Gelenk_D", 3),
    ]
    .map(|(n, i)| (n.to_owned(), i));
    let table = skin_table_slots(&o3d, Some(&slots[..]))?;
    let axis = model_dir_from_omsi(trailer_end_axis(COUPLING, HITCH)?);
    let band = trailer_end_vertices(&model, axis);
    Some(Bellow {
        model,
        omsi,
        table,
        band,
    })
}

/// The posed position of a model-space point: the weighted sum of the joint matrices.
fn posed(entry: &SkinEntry, mats: &[Mat4], p: Vec3) -> Vec3 {
    let (joints, weights) = entry;
    (0..4).fold(Vec3::ZERO, |acc, k| {
        acc + weights[k] * mats[usize::from(joints[k])].transform_point3(p)
    })
}

fn identity_mats() -> Vec<Mat4> {
    vec![Mat4::IDENTITY; usize::from(TRAILER_JOINT) + 1]
}

#[test]
fn kajosoft_bellow_remaps_its_75_trailer_end_vertices() {
    let Some(b) = load() else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    assert_eq!(b.band.len(), 75);
    let raw_axis = raw_dir_from_omsi(trailer_end_axis(COUPLING, HITCH).unwrap());
    let raw: Vec<Vec3> = b.omsi.iter().map(|o| Vec3::new(o.x, o.z, o.y)).collect();
    assert_eq!(trailer_end_vertices(&raw, raw_axis), b.band);
    let (out, n) = remap_trailer_end(&b.table, &b.band, RING_D, TRAILER_JOINT);
    assert_eq!(n, 75);
    let changed: Vec<usize> = (0..b.table.len())
        .filter(|&i| out[i] != b.table[i])
        .collect();
    assert!(changed.iter().all(|i| b.band.contains(i)));
}

#[test]
fn kajosoft_rest_pose_is_the_authored_geometry() {
    let Some(b) = load() else {
        return;
    };
    let (out, _) = remap_trailer_end(&b.table, &b.band, RING_D, TRAILER_JOINT);
    let mats = identity_mats();
    for (entry, p) in out.iter().zip(&b.model) {
        let p_posed = posed(entry, &mats, *p);
        assert!(p_posed.abs_diff_eq(*p, 1e-6), "{p_posed:?} != {p:?}");
    }
}
