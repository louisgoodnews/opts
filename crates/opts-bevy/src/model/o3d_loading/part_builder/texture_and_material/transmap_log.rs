//! Load log of transmap compositions: the mesh, the textures, the measured opaque share and the
//! body and glass passes chosen (`grep "transmap composition"` in the debug log).

use super::*;

/// Whether a `[matl_alpha]` mode asks for the body and glass split (blend mode 2, or none).
pub(super) fn split_requested(matl_alpha: Option<u32>) -> bool {
    matl_alpha.is_none_or(|a| a == 2)
}

/// The alpha mode the body pass of a transmap texture gets (see `configure_slot_material`).
fn body_alpha_label(glass: bool, matl_alpha: Option<u32>) -> &'static str {
    match matl_alpha {
        _ if glass => "mask_0.95",
        Some(1) => "mask_0.5",
        Some(0) => "o3d_diffuse",
        _ => "blend",
    }
}

/// One transmap composition. `share` is `None` on a cache hit (the first composition logged it).
pub(super) fn log_transmap_pass(
    ctx: &SlotBuildCtx<'_>,
    tex_name: Option<&str>,
    trans_map: &str,
    matl_alpha: Option<u32>,
    share: Option<f32>,
    glass: bool,
    cached: bool,
) {
    tracing::debug!(
        mesh = %ctx.o3d_path.display(),
        texture = ?tex_name,
        trans_map,
        matl_alpha = ?matl_alpha,
        opaque_share = ?share,
        split_threshold = TRANSMAP_SPLIT_SHARE,
        split_requested = split_requested(matl_alpha),
        pass = if glass { "split" } else { "single" },
        body_alpha = body_alpha_label(glass, matl_alpha),
        cached,
        "transmap composition"
    );
}
