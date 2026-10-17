//! Glass and mirror surfaces: what the model loader recognised, and the glass look that follows
//! [`LightQuality::glass_reflections`] (`docs/src/architecture/reflections-research.md` section 7,
//! step B; `lighting-system.md` section 9).
//!
//! * The loader calls [`note_surface`] for every material it builds (a string test, no
//!   allocation unless the name matches). Glass is a **blended** material whose texture name is
//!   glass-like ([`is_glass_name`]) or the glass pass of a transmap split; mirrors are materials
//!   whose texture is a mirror pane ([`is_mirror_name`], `reflexion<N>.bmp`). Both go into a process-wide
//!   registry keyed by the material's asset id, because the loader has no ECS access.
//! * [`sync_glass`] makes the registered glass materials follow the quality: with
//!   `glass_reflections` on they get a low roughness, a higher reflectance and
//!   [`AlphaMode::Premultiplied`] (the lit specular is added at full strength instead of being
//!   faded with the texture alpha, so a transparent window still mirrors the sky); with it off the
//!   original values come back exactly. At most [`MAX_PER_FRAME`] materials change per frame.
//!
//! [`mirrors`](crate::mirrors) reads the registry to find the mirror meshes of the player vehicle.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use bevy::prelude::*;

use crate::light_quality::{LightQuality, LightQualitySet};

/// Perceptual roughness of glass with reflections on.
pub const GLASS_ROUGHNESS: f32 = 0.06;
/// Reflectance (Fresnel `F0` scale) of glass with reflections on.
pub const GLASS_REFLECTANCE: f32 = 0.6;
/// Factor on the glass base colour with reflections on: under premultiplied alpha the lit diffuse
/// colour is added at full weight, so a bright window texture would turn milky; halving it keeps
/// the tint and lets the reflection and the view through dominate.
pub const GLASS_TINT: f32 = 0.5;
/// Most materials switched per frame (a district of buildings registers thousands).
pub const MAX_PER_FRAME: usize = 256;

/// What a registered surface is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    /// Blended, glass-like.
    Glass,
    /// A mirror texture.
    Mirror,
}

/// The values glass had before reflections touched it.
#[derive(Debug, Clone, Copy)]
struct GlassOriginal {
    alpha: AlphaMode,
    roughness: f32,
    reflectance: f32,
    base_color: Color,
}

#[derive(Debug, Clone)]
struct Entry {
    kind: SurfaceKind,
    original: GlassOriginal,
    /// Whether the glass look is applied now.
    applied: bool,
}

type Registry = HashMap<AssetId<StandardMaterial>, Entry>;

static REGISTRY: Mutex<Option<Registry>> = Mutex::new(None);
/// Bumped on every registration; [`sync_glass`] looks at the registry only when it moved.
static REVISION: AtomicU64 = AtomicU64::new(0);

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
    insert(handle.id(), kind, material);
}

fn insert(id: AssetId<StandardMaterial>, kind: SurfaceKind, m: &StandardMaterial) {
    if let Ok(mut g) = REGISTRY.lock() {
        g.get_or_insert_with(HashMap::new)
            .entry(id)
            .or_insert(Entry {
                kind,
                original: GlassOriginal {
                    alpha: m.alpha_mode,
                    roughness: m.perceptual_roughness,
                    reflectance: m.reflectance,
                    base_color: m.base_color,
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
    insert(handle.id(), kind, material);
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

/// The glass look applied to `m`, from the values it had before.
fn apply_look(m: &mut StandardMaterial, original: &GlassOriginal) {
    m.perceptual_roughness = original.roughness.min(GLASS_ROUGHNESS);
    m.reflectance = original.reflectance.max(GLASS_REFLECTANCE);
    m.alpha_mode = AlphaMode::Premultiplied;
    let c = original.base_color.to_linear();
    m.base_color = Color::linear_rgba(
        c.red * GLASS_TINT,
        c.green * GLASS_TINT,
        c.blue * GLASS_TINT,
        c.alpha,
    );
}

fn restore(m: &mut StandardMaterial, original: &GlassOriginal) {
    m.perceptual_roughness = original.roughness;
    m.reflectance = original.reflectance;
    m.alpha_mode = original.alpha;
    m.base_color = original.base_color;
}

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
    mut stats: ResMut<GlassStats>,
    mut seen: Local<(u64, Option<bool>)>,
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
        let Some(mut m) = materials.get_mut(*id) else {
            // Not in this store (another session's material, or already freed): nothing to do.
            continue;
        };
        if want {
            apply_look(&mut m, &entry.original);
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

/// Registers [`sync_glass`].
#[derive(Default)]
pub struct GlassReflectionsPlugin;

impl Plugin for GlassReflectionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightQuality>()
            .init_resource::<GlassStats>()
            .add_systems(
                Update,
                sync_glass
                    .after(LightQualitySet)
                    .run_if(resource_exists::<Assets<StandardMaterial>>),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blended() -> StandardMaterial {
        StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.8,
            reflectance: 0.5,
            base_color: Color::srgba(0.8, 0.9, 1.0, 0.4),
            ..default()
        }
    }

    #[test]
    fn names_are_classified() {
        assert_eq!(classify("Fenster.tga", true), Some(SurfaceKind::Glass));
        assert_eq!(classify("Fenster.tga", false), None, "opaque is not glass");
        assert_eq!(
            classify("Texture\\Reflexion0.bmp", false),
            Some(SurfaceKind::Mirror)
        );
        assert_eq!(classify("reflexion3.bmp", true), Some(SurfaceKind::Mirror));
        assert_eq!(classify("reflexion12.bmp", true), Some(SurfaceKind::Mirror));
        // The housing of a mirror unit is not the pane.
        assert_eq!(classify("nlc_side_mirror.dds", true), None);
        assert_eq!(classify("sl_mirrors.dds", false), None);
        assert_eq!(classify("o3d_spiegel.dds", true), None);
        assert_eq!(classify("reflexion.bmp", false), None, "needs its number");
        assert_eq!(classify("reflexion_x.bmp", false), None);
        assert_eq!(classify("verkehrsspiegel.bmp", false), None);
        assert_eq!(classify("karosserie.bmp", true), None);
    }

    fn test_app() -> App {
        let mut app = App::new();
        app.init_resource::<Assets<StandardMaterial>>()
            .insert_resource(LightQuality::default())
            .init_resource::<GlassStats>()
            .add_systems(Update, sync_glass);
        app
    }

    #[test]
    fn glass_follows_the_setting_and_restores_exactly() {
        let mut app = test_app();
        let h = unique_material(
            &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
            blended(),
        );
        let original = blended();
        note_surface(&h, Some("fenster_int.tga"), &original, false);
        assert_eq!(surface_kind(h.id()), Some(SurfaceKind::Glass));

        app.update();
        {
            let mats = app.world().resource::<Assets<StandardMaterial>>();
            let m = mats.get(&h).unwrap();
            assert_eq!(m.alpha_mode, AlphaMode::Premultiplied);
            assert!(m.perceptual_roughness <= GLASS_ROUGHNESS + 1e-6);
            assert!(m.reflectance >= GLASS_REFLECTANCE - 1e-6);
            assert!(m.base_color.to_linear().red < original.base_color.to_linear().red);
            assert!((m.base_color.alpha() - 0.4).abs() < 1e-6, "alpha untouched");
        }

        app.world_mut()
            .resource_mut::<LightQuality>()
            .glass_reflections = false;
        app.update();
        let mats = app.world().resource::<Assets<StandardMaterial>>();
        let m = mats.get(&h).unwrap();
        assert_eq!(m.alpha_mode, AlphaMode::Blend);
        assert_eq!(m.perceptual_roughness, 0.8);
        assert_eq!(m.reflectance, 0.5);
        assert_eq!(m.base_color, original.base_color);
    }

    #[test]
    fn with_reflections_off_a_new_glass_material_stays_untouched() {
        let mut app = test_app();
        app.world_mut()
            .resource_mut::<LightQuality>()
            .glass_reflections = false;
        let h = unique_material(
            &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
            blended(),
        );
        note_surface(&h, Some("glass.tga"), &blended(), false);
        app.update();
        let m = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&h)
            .unwrap();
        assert_eq!(m.alpha_mode, AlphaMode::Blend);
        assert_eq!(m.perceptual_roughness, 0.8);
    }

    #[test]
    fn the_glass_pass_of_a_transmap_split_is_glass_by_force() {
        let mut app = test_app();
        let h = unique_material(
            &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
            blended(),
        );
        note_surface(&h, Some("bus_body.bmp"), &blended(), true);
        assert_eq!(surface_kind(h.id()), Some(SurfaceKind::Glass));
        let opaque = StandardMaterial::default();
        let h2 = unique_material(
            &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
            opaque.clone(),
        );
        note_surface(&h2, Some("fenster.tga"), &opaque, false);
        assert_eq!(
            surface_kind(h2.id()),
            None,
            "an opaque window is not touched"
        );
    }
}
