//! Making the registered glass follow the light quality each frame (`sync_glass`), and the
//! stats it reports.

use super::*;

/// What [`sync_glass`] reports (`sim.get` shows it).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GlassStats {
    /// Glass materials registered by the loader.
    pub registered: usize,
    /// Glass materials currently showing the reflective look.
    pub reflective: usize,
}

/// Makes the registered glass follow [`LightQuality::glass_reflections`].
pub fn sync_glass(
    quality: Res<LightQuality>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: Option<ResMut<Assets<Image>>>,
    mut stats: ResMut<GlassStats>,
    mut seen: Local<(u64, Option<bool>)>,
    mut copies: Local<HashMap<AssetId<Image>, AssetId<Image>>>,
) {
    let want = quality.glass_reflections;
    let revision = REVISION.load(Ordering::Relaxed);
    let pending_before = seen.1 != Some(want) || seen.0 != revision;
    if !pending_before {
        return;
    }
    let Ok(mut guard) = REGISTRY.lock() else {
        return;
    };
    let Some(map) = guard.as_mut() else { return };
    let mut changed = 0usize;
    let mut left = false;
    for (id, entry) in map.iter_mut() {
        if entry.kind != SurfaceKind::Glass || entry.applied == want {
            continue;
        }
        if changed >= MAX_PER_FRAME {
            left = true;
            break;
        }
        if materials.get(*id).is_none() {
            // Not in this store (another session's material, or already freed): nothing to do.
            continue;
        }
        let texture = if want {
            premultiplied_texture(&entry.original, images.as_deref_mut(), &mut copies)
        } else {
            None
        };
        let Some(mut m) = materials.get_mut(*id) else {
            continue;
        };
        if want {
            apply_look(&mut m, &entry.original, texture);
        } else {
            restore(&mut m, &entry.original);
        }
        entry.applied = want;
        changed += 1;
    }
    stats.registered = map
        .values()
        .filter(|e| e.kind == SurfaceKind::Glass)
        .count();
    stats.reflective = map
        .values()
        .filter(|e| e.kind == SurfaceKind::Glass && e.applied)
        .count();
    // Done when nothing is left over: wait for the next change.
    *seen = if left {
        (revision.wrapping_sub(1), seen.1)
    } else {
        (revision, Some(want))
    };
}

/// The premultiplied copy of the glass's colour texture, built once per source image (the copies
/// are held weakly here: the materials that show them keep them alive).
pub(super) fn premultiplied_texture(
    original: &GlassOriginal,
    images: Option<&mut Assets<Image>>,
    copies: &mut HashMap<AssetId<Image>, AssetId<Image>>,
) -> Option<Handle<Image>> {
    let source = original.texture.as_ref()?;
    let images = images?;
    if let Some(copy) = copies.get(&source.id()) {
        if let Some(strong) = images.get_strong_handle(*copy) {
            return Some(strong);
        }
    }
    let copy = premultiplied_copy(images.get(source)?)?;
    let handle = images.add(copy);
    copies.insert(source.id(), handle.id());
    Some(handle)
}
