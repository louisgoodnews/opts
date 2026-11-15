//! The public entry points that build the animation steps of a model, and their warning.

use super::*;

/// The steps of one `[newanim]` block. `pivot` is the mesh file's own origin (the translation of
/// its matrix chunk); it is used for `origin_from_mesh` unless an explicit `origin_trans` is given.
/// The mesh's matrix rotation is not known here (identity); use [`anim_steps_with`] to pass it.
#[must_use]
pub fn anim_steps(anim: &ModelAnim, pivot: Option<Vec3>) -> Vec<AnimStep> {
    steps_of(anim, pivot.map(MeshPivot::plain))
}

/// [`anim_steps`] with the mesh's pivot and matrix rotation.
#[must_use]
pub fn anim_steps_with(anim: &ModelAnim, pivot: Option<MeshPivot>) -> Vec<AnimStep> {
    steps_of(anim, pivot)
}

/// Logs once per mesh that its steering/rotation axis was composed with the mesh matrix.
pub(super) fn warn_matrix_once(mesh: &str) {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(Mutex::default);
    if seen.lock().is_ok_and(|mut s| s.insert(mesh.to_lowercase())) {
        tracing::warn!(
            mesh,
            "origin_from_mesh rotation axis composed with the mesh matrix rotation (animation-axes.md)"
        );
    }
}

/// OMSI axes (x right, y forward, z up) to Bevy axes (x right, y up, z backward).
pub(super) fn omsi_to_bevy_basis() -> Mat4 {
    Mat4::from_cols(
        Vec3::X.extend(0.0),
        (-Vec3::Z).extend(0.0),
        Vec3::Y.extend(0.0),
        Vec4::W,
    )
}
