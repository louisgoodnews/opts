//! The trailer-front joint on the Kajosoft Citybus 530 Facelift bellow (`przegub_zew.o3d`): the
//! trailer-end band is 75 vertices, all on ring D, and the remap keeps the rest pose. Skipped
//! without `OMSI_PATH`.

use std::path::PathBuf;

use bevy::math::{Mat4, Vec3};
use opts_bevy::mesh::{remap_trailer_end, skin_table_slots, trailer_end_vertices};

const RING_D: u16 = 3;
const TRAILER_JOINT: u16 = 5;

#[test]
fn kajosoft_bellow_remaps_its_75_trailer_end_vertices() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let path = root.join("Vehicles/Citybus 530 Facelift by Kajosoft/Model/C1_G/przegub_zew.o3d");
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let o3d = opts_core::formats::o3d::parse(&bytes).expect("przegub_zew.o3d");
    let positions: Vec<Vec3> = o3d.vertices.iter().map(|v| v.position).collect();
    let slots = [
        ("Gelenk_A", 0),
        ("Gelenk_B", 1),
        ("Gelenk_C", 2),
        ("Gelenk_D", 3),
    ]
    .map(|(n, i)| (n.to_owned(), i));
    let table = skin_table_slots(&o3d, Some(&slots[..])).expect("skinned bellow");
    let band = trailer_end_vertices(&positions, 2);
    assert_eq!(band.len(), 75);
    let (out, n) = remap_trailer_end(&table, &band, RING_D, TRAILER_JOINT);
    assert_eq!(n, 75);
    let changed: Vec<usize> = (0..table.len()).filter(|&i| out[i] != table[i]).collect();
    assert!(changed.iter().all(|i| band.contains(i)));
    let mats = vec![Mat4::IDENTITY; usize::from(TRAILER_JOINT) + 1];
    for (entry, p) in out.iter().zip(&positions) {
        let posed = entry
            .0
            .iter()
            .zip(&entry.1)
            .fold(Vec3::ZERO, |acc, (j, w)| {
                acc + *w * mats[usize::from(*j)].transform_point3(*p)
            });
        assert!(posed.abs_diff_eq(*p, 1e-6), "{posed:?} != {p:?}");
    }
}
