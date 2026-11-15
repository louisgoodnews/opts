//! Surface names: which texture names are glass and which are mirror panes, and the
//! classification of a texture (given whether its material blends).

use super::*;

/// Glass by texture name (the corpus has no other marker; see the research report).
#[must_use]
pub fn is_glass_name(name: &str) -> bool {
    [
        "glas",
        "glass",
        "scheibe",
        "fenster",
        "window",
        "verglas",
        "windscreen",
        "windshield",
    ]
    .iter()
    .any(|w| name.contains(w))
}

/// `N` of a mirror pane texture `reflexion<N>.bmp` (any case, with or without folders).
#[must_use]
pub fn mirror_index_of(texture: &str) -> Option<u32> {
    let lower = texture.to_lowercase();
    let stem = lower.rsplit(['/', '\\']).next().unwrap_or(&lower);
    if !is_mirror_name(stem) {
        return None;
    }
    stem.strip_prefix("reflexion")?
        .split('.')
        .next()?
        .parse()
        .ok()
}

/// A mirror pane by texture name: OMSI's `reflexion<N>.bmp` convention (N = 0..5, the pane of
/// one mirror; 807 mesh files of 92 vehicle folders of the reference installation use it). The
/// mirror *housing* of such meshes carries a different texture (`nlc_side_mirror.dds`,
/// `sl_mirrors.dds`, `aussenspiegel.bmp`, ...), which is why the name test is on the pane texture
/// and not on `mirror` / `spiegel`.
#[must_use]
pub fn is_mirror_name(name: &str) -> bool {
    name.strip_prefix("reflexion")
        .and_then(|rest| rest.split('.').next())
        .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
}

/// The surface kind of a texture file name (any case, with or without folders), given whether
/// the material blends: mirrors win over glass, glass needs a blended material.
#[must_use]
pub fn classify(texture: &str, blended: bool) -> Option<SurfaceKind> {
    let lower = texture.to_lowercase();
    let stem = lower.rsplit(['/', '\\']).next().unwrap_or(&lower);
    if is_mirror_name(stem) {
        Some(SurfaceKind::Mirror)
    } else if blended && is_glass_name(stem) {
        Some(SurfaceKind::Glass)
    } else {
        None
    }
}
