//! The camera gate of a definition: which viewpoint it plays from.

/// Which camera a `[viewpoint]` allows: the interior sounds have 2 (and 6), the exterior ones 5 (and
/// 1); every other value (and none) plays everywhere. Read from the file names: in the installed
/// vehicles 5,586 of the 11,163 definitions with 5 are named `ext`, `außen` ... (the Citybus 530's
/// `OM_906hLA_E2_ext_*` engine loops and `rear_piep_piep_ext` have 5, the engine loops without
/// `ext` have 2), only 29 of the 13,033 with 2 are; 88 of the 96 with 1 are named `eng_out_*` or
/// the like, 28 of the 38 with 6 `engine_in_*`. ❓ Q-SND-004: the values 3 and 4 are unknown.
#[must_use]
pub fn viewpoint_allows(viewpoint: Option<i32>, camera_inside: bool) -> bool {
    match viewpoint {
        Some(2 | 6) => camera_inside,
        Some(1 | 5) => !camera_inside,
        _ => true,
    }
}
