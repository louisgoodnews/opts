//! Checks an O3D mesh after parsing: the encryption key, the vertex layout and the plausible
//! extent.

use super::*;

pub(super) fn decode_and_validate_o3d(
    models: &mut [O3d],
    o3d_path: &Path,
    sc: &mut TraceScratch,
    cache: &mut ContentCache,
) -> bool {
    let is_o3d = o3d_path
        .extension()
        .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("o3d"));
    if !is_o3d {
        return true;
    }
    for m in models.iter_mut() {
        if m.encryption_key.is_some() && o3d::exact::decode(m).is_some() {
            cache.stats.exact_meshes += 1;
            sc.stage = "exact";
        }
    }
    sc.decode_ms = sc.lap.split();
    if models
        .iter()
        .any(|m| matches!(m.vertex_layout(), o3d::VertexLayout::Unreadable))
    {
        sc.stage = "rejected";
        cache.stats.protected_meshes += 1;
        cache.protected.insert(o3d_path.to_path_buf());
        tracing::warn!(path = %o3d_path.display(), "mesh vertex positions are scrambled and could not be decoded; skipped");
        return false;
    }
    let extent = models.iter().map(model_extent).fold(0.0_f32, f32::max);
    if extent > 60.0 {
        tracing::info!(path = %o3d_path.display(), extent, layout = ?models.first().map(O3d::vertex_layout), "large mesh");
    }
    if extent > MAX_PLAUSIBLE_EXTENT_M {
        sc.stage = "extent";
        sc.warn(|| format!("extent {extent:.0} m exceeds {MAX_PLAUSIBLE_EXTENT_M:.0} m"));
        cache.stats.protected_meshes += 1;
        cache.protected.insert(o3d_path.to_path_buf());
        tracing::warn!(path = %o3d_path.display(), extent, "mesh extent is implausible; skipped");
        return false;
    }
    if models.iter().any(|m| {
        matches!(
            m.vertex_layout(),
            o3d::VertexLayout::AxisCycled | o3d::VertexLayout::RecomputedNormals
        )
    }) {
        cache.stats.axis_cycled_meshes += 1;
        tracing::debug!(path = %o3d_path.display(), "axis-cycled vertex layout; normals recomputed");
    }
    true
}

fn model_extent(m: &O3d) -> f32 {
    let (mut lo, mut hi) = (
        bevy::math::Vec3::splat(f32::MAX),
        bevy::math::Vec3::splat(f32::MIN),
    );
    for t in &m.indices {
        for &i in t {
            if let Some(v) = m.vertices.get(i as usize) {
                lo = lo.min(v.position);
                hi = hi.max(v.position);
            }
        }
    }
    (hi - lo).max_element()
}
