//! The process-wide surface registry the model loader writes to: registration of glass and
//! mirror materials by asset id, and the lookups of what a material is.

use super::*;

/// A handle with a process-unique id for `material`, added to `materials` (tests: the registry
/// is process-wide and the index ids of separate test apps would collide).
#[cfg(test)]
pub(crate) fn unique_material(
    materials: &mut Assets<StandardMaterial>,
    material: StandardMaterial,
) -> Handle<StandardMaterial> {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let id = bevy::asset::uuid::Uuid::from_u128(u128::from(NEXT.fetch_add(1, Ordering::Relaxed)));
    let handle = Handle::<StandardMaterial>::Uuid(id, Default::default());
    materials.insert(&handle, material).unwrap();
    handle
}

/// Called by the model loader for every material it creates. `force_glass` marks the glass pass
/// of a transmap split (glass whatever the texture is called).
pub fn note_surface(
    handle: &Handle<StandardMaterial>,
    texture: Option<&str>,
    material: &StandardMaterial,
    force_glass: bool,
) {
    let blended = matches!(material.alpha_mode, AlphaMode::Blend);
    let kind = if force_glass && blended {
        Some(SurfaceKind::Glass)
    } else {
        texture.and_then(|t| classify(t, blended))
    };
    let Some(kind) = kind else { return };
    let index = texture
        .filter(|_| kind == SurfaceKind::Mirror)
        .and_then(mirror_index_of);
    insert(handle.id(), kind, index, material);
}

pub(super) fn insert(
    id: AssetId<StandardMaterial>,
    kind: SurfaceKind,
    mirror_index: Option<u32>,
    m: &StandardMaterial,
) {
    if let Ok(mut g) = REGISTRY.lock() {
        g.get_or_insert_with(HashMap::new)
            .entry(id)
            .or_insert(Entry {
                kind,
                mirror_index,
                original: GlassOriginal {
                    alpha: m.alpha_mode,
                    roughness: m.perceptual_roughness,
                    reflectance: m.reflectance,
                    base_color: m.base_color,
                    texture: m.base_color_texture.clone(),
                },
                applied: false,
            });
        REVISION.fetch_add(1, Ordering::Relaxed);
    }
}

/// Registers `material` as `kind` directly (tests and tools that build materials without the
/// loader).
pub fn register_surface(
    handle: &Handle<StandardMaterial>,
    kind: SurfaceKind,
    material: &StandardMaterial,
) {
    insert(handle.id(), kind, None, material);
}

/// Gives a registered mirror material the pane number `index` (tests).
#[cfg(test)]
pub(crate) fn set_mirror_index(id: AssetId<StandardMaterial>, index: u32) {
    if let Ok(mut g) = REGISTRY.lock() {
        if let Some(e) = g.as_mut().and_then(|m| m.get_mut(&id)) {
            e.mirror_index = Some(index);
        }
    }
}

/// `N` of the `reflexion<N>.bmp` pane texture of a registered mirror material.
#[must_use]
pub fn mirror_index(id: AssetId<StandardMaterial>) -> Option<u32> {
    REGISTRY.lock().ok()?.as_ref()?.get(&id)?.mirror_index
}

/// What the registry knows about a material.
#[must_use]
pub fn surface_kind(id: AssetId<StandardMaterial>) -> Option<SurfaceKind> {
    REGISTRY.lock().ok()?.as_ref()?.get(&id).map(|e| e.kind)
}

/// How many surfaces of each kind are registered: `(glass, mirrors)`.
#[must_use]
pub fn registered_counts() -> (usize, usize) {
    let g = REGISTRY.lock();
    let Ok(g) = g else { return (0, 0) };
    let Some(map) = g.as_ref() else { return (0, 0) };
    let glass = map
        .values()
        .filter(|e| e.kind == SurfaceKind::Glass)
        .count();
    (glass, map.len() - glass)
}
