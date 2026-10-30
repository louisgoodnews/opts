//! Unit tests for `super` (`ground.rs`).

use super::painted_layers;

#[test]
fn painted_layers_come_from_mask_names_in_either_folder_and_case() {
    let dir = std::env::temp_dir().join(format!("opts_painted_layers_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("Texture/MAP")).unwrap();
    for name in [
        "Texture/MAP/tile_1_2.map.16.dds",
        "Texture/MAP/tile_1_3.map.9.dds",
        "Texture/tile_1_2.map.3.dds",
        "Texture/MAP/tile_1_2.map.198.dds",
        "Texture/water.tga",
    ] {
        std::fs::write(dir.join(name), b"x").unwrap();
    }
    // Layer 198 is beyond the 21 listed layers (a stale mask) and is ignored.
    assert_eq!(painted_layers(&dir, 21), vec![3, 9, 16]);
    let _ = std::fs::remove_dir_all(&dir);
}
