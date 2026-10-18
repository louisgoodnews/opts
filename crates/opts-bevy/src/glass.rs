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
//! * **Premultiplied glass:** `AlphaMode::Premultiplied` adds the lit colour at full weight and
//!   fades only what is behind it by `1 - alpha`. Left alone, the diffuse colour of a window
//!   texture (a pale tint, white print, frames) is added over the view through the glass at any
//!   transparency: the milk-glass look of the Hamburg buses. The look therefore swaps the
//!   texture for a copy whose colour is multiplied by its alpha ([`premultiply_rgba8`], in
//!   linear light), so the diffuse part is faded exactly as `Blend` fades it while the specular
//!   reflection stays at full strength. The copy is shared between the materials of a texture
//!   and dropped with them; the original comes back when the look is switched off.
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
/// Factor on the glass base colour with reflections on, after the colour was multiplied by its
/// alpha (see the module documentation): 1 keeps the tint of the texture exactly as `Blend`
/// draws it. A glass without a texture (or one whose texture cannot be read) still gets
/// [`GLASS_FALLBACK_TINT`] instead.
pub const GLASS_TINT: f32 = 1.0;
/// Factor on the base colour of glass whose texture cannot be premultiplied (block-compressed
/// formats the CPU decoder does not know): the diffuse part is added at full weight, so it is kept
/// small.
pub const GLASS_FALLBACK_TINT: f32 = 0.15;
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
#[derive(Debug, Clone)]
struct GlassOriginal {
    alpha: AlphaMode,
    roughness: f32,
    reflectance: f32,
    base_color: Color,
    /// The colour texture before it was premultiplied (held strongly while the copy is shown).
    texture: Option<Handle<Image>>,
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

/// Multiplies the colour of RGBA8 texels by their alpha, in linear light (`srgb`: the bytes are
/// sRGB encoded). Texels with alpha 255 are left alone, with alpha 0 they become black.
pub fn premultiply_rgba8(data: &mut [u8], srgb: bool) {
    let to_linear = |v: u8| -> f32 {
        let c = f32::from(v) / 255.0;
        if !srgb {
            c
        } else if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let from_linear = |l: f32| -> u8 {
        let c = if !srgb {
            l
        } else if l <= 0.003_130_8 {
            l * 12.92
        } else {
            1.055 * l.powf(1.0 / 2.4) - 0.055
        };
        (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
    };
    // Cache of the last texel: window textures are flat in large areas.
    let mut last: Option<([u8; 4], [u8; 3])> = None;
    for px in data.chunks_exact_mut(4) {
        let a = px[3];
        if a == 255 {
            continue;
        }
        let key = [px[0], px[1], px[2], a];
        if let Some((k, out)) = last {
            if k == key {
                px[..3].copy_from_slice(&out);
                continue;
            }
        }
        let f = f32::from(a) / 255.0;
        let out = [
            from_linear(to_linear(px[0]) * f),
            from_linear(to_linear(px[1]) * f),
            from_linear(to_linear(px[2]) * f),
        ];
        px[..3].copy_from_slice(&out);
        last = Some((key, out));
    }
}

/// A copy of `source` whose colour is multiplied by its alpha, with a fresh mip chain; `None`
/// when the texels cannot be read on the CPU.
fn premultiplied_copy(source: &Image) -> Option<Image> {
    use bevy::render::render_resource::TextureFormat as F;
    let mut out = crate::texture::decode_to_rgba8(source)?;
    let srgb = match out.texture_descriptor.format {
        F::Rgba8UnormSrgb => true,
        F::Rgba8Unorm => false,
        _ => return None,
    };
    let texels = out.width() as usize * out.height() as usize * 4;
    let data = out.data.as_mut()?;
    data.truncate(texels);
    premultiply_rgba8(data, srgb);
    crate::texture_filter::prepare_object_texture(&mut out);
    Some(out)
}

/// The glass look applied to `m`, from the values it had before. `texture` is the premultiplied
/// copy of the colour texture, when there is one.
fn apply_look(m: &mut StandardMaterial, original: &GlassOriginal, texture: Option<Handle<Image>>) {
    m.perceptual_roughness = original.roughness.min(GLASS_ROUGHNESS);
    m.reflectance = original.reflectance.max(GLASS_REFLECTANCE);
    m.alpha_mode = AlphaMode::Premultiplied;
    let c = original.base_color.to_linear();
    // The material's own alpha multiplies the texture alpha: it belongs in the colour as well.
    let (tint, fade) = match (&original.texture, &texture) {
        (Some(_), None) => (GLASS_FALLBACK_TINT, 1.0),
        _ => (GLASS_TINT, c.alpha),
    };
    let k = tint * fade;
    m.base_color = Color::linear_rgba(c.red * k, c.green * k, c.blue * k, c.alpha);
    if let Some(t) = texture {
        m.base_color_texture = Some(t);
    }
}

fn restore(m: &mut StandardMaterial, original: &GlassOriginal) {
    m.perceptual_roughness = original.roughness;
    m.reflectance = original.reflectance;
    m.alpha_mode = original.alpha;
    m.base_color = original.base_color;
    m.base_color_texture = original.texture.clone();
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
fn premultiplied_texture(
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

    #[test]
    fn premultiplying_fades_the_colour_by_alpha_in_linear_light() {
        // A white texel at alpha 128 (about 0.5) is sRGB 188 once multiplied in linear light, not 128.
        let mut px = [255, 255, 255, 128, 10, 20, 30, 255, 200, 100, 50, 0];
        premultiply_rgba8(&mut px, true);
        assert!((i32::from(px[0]) - 188).abs() <= 1, "{}", px[0]);
        assert_eq!(px[3], 128, "alpha is not touched");
        assert_eq!(&px[4..8], &[10, 20, 30, 255], "opaque texels stay");
        assert_eq!(&px[8..11], &[0, 0, 0], "transparent texels turn black");
        let mut linear = [200, 100, 50, 128];
        premultiply_rgba8(&mut linear, false);
        assert_eq!(&linear[..3], &[100, 50, 25]);
    }

    fn window_image(rgba: [u8; 4]) -> Image {
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        let data: Vec<u8> = (0..16).flat_map(|_| rgba).collect();
        Image::new(
            Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            default(),
        )
    }

    /// The milk-glass bug of the Hamburg buses: a pale, mostly transparent window texture added
    /// at full weight by `Premultiplied` turned the view through the glass white.
    #[test]
    fn a_pale_window_texture_is_faded_by_its_alpha_while_the_look_is_on() {
        let mut app = test_app();
        app.init_resource::<Assets<Image>>();
        let texture = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(window_image([230, 235, 240, 40]));
        let material = StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            base_color_texture: Some(texture.clone()),
            ..default()
        };
        let h = unique_material(
            &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
            material.clone(),
        );
        note_surface(&h, Some("fenster.tga"), &material, false);
        app.update();
        let shown = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&h)
            .unwrap()
            .base_color_texture
            .clone()
            .expect("texture");
        assert_ne!(shown.id(), texture.id(), "the premultiplied copy is shown");
        let images = app.world().resource::<Assets<Image>>();
        let copy = images.get(&shown).unwrap();
        let px = &copy.data.as_ref().unwrap()[..4];
        assert!(px[0] < 120, "diffuse added over the view must be faint: {px:?}");
        assert_eq!(px[3], 40);
        assert_eq!(images.get(&texture).unwrap().data.as_ref().unwrap()[0], 230);

        app.world_mut()
            .resource_mut::<LightQuality>()
            .glass_reflections = false;
        app.update();
        let back = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&h)
            .unwrap();
        assert_eq!(back.base_color_texture.as_ref().map(|t| t.id()), Some(texture.id()));
        assert_eq!(back.alpha_mode, AlphaMode::Blend);
    }

    #[test]
    fn glass_whose_texture_cannot_be_read_gets_a_small_diffuse_instead() {
        let mut app = test_app();
        app.init_resource::<Assets<Image>>();
        let missing = Handle::<Image>::Uuid(bevy::asset::uuid::Uuid::from_u128(99), default());
        let material = StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            base_color_texture: Some(missing.clone()),
            ..default()
        };
        let h = unique_material(
            &mut app.world_mut().resource_mut::<Assets<StandardMaterial>>(),
            material.clone(),
        );
        note_surface(&h, Some("glas.tga"), &material, false);
        app.update();
        let m = app
            .world()
            .resource::<Assets<StandardMaterial>>()
            .get(&h)
            .unwrap();
        assert_eq!(m.alpha_mode, AlphaMode::Premultiplied);
        assert_eq!(m.base_color_texture.as_ref().map(|t| t.id()), Some(missing.id()));
        assert!((m.base_color.to_linear().red - GLASS_FALLBACK_TINT).abs() < 1e-6);
    }
}
