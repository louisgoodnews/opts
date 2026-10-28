//! Unit tests for `super` (`segments.rs`).

use super::mesh_stem;

#[test]
fn copies_in_other_folders_share_a_stem() {
    assert_eq!(
        mesh_stem("VDV\\BHD_LC_VDV_Display.o3d"),
        "bhd_lc_vdv_display"
    );
    assert_eq!(
        mesh_stem("VDV\\Display\\BHD_LC_VDV_Display.o3d"),
        "bhd_lc_vdv_display"
    );
    assert_eq!(mesh_stem("plain.x"), "plain");
    assert_eq!(mesh_stem("noext"), "noext");
}
