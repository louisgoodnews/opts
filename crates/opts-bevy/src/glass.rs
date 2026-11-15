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
/// How much of the model's glass tint and opacity is kept with reflections on (colour and alpha
/// are scaled together, so the pane stays consistently premultiplied). Many models paint their
/// windows as a light diffuse colour at partial alpha; blended over the interior that is a milky
/// veil, and the reflections the look adds come on top of it. At this value the pane is clearly
/// transparent: the view through it keeps most of its contrast and the reflection of the sky
/// and the sun is what shows on it.
pub const GLASS_CLARITY: f32 = 0.45;
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
    /// `N` of a `reflexion<N>.bmp` mirror pane.
    mirror_index: Option<u32>,
    original: GlassOriginal,
    /// Whether the glass look is applied now.
    applied: bool,
}

type Registry = HashMap<AssetId<StandardMaterial>, Entry>;

static REGISTRY: Mutex<Option<Registry>> = Mutex::new(None);
/// Bumped on every registration; [`sync_glass`] looks at the registry only when it moved.
static REVISION: AtomicU64 = AtomicU64::new(0);

mod names;
pub use names::*;

mod registry;
pub use registry::*;

mod premultiply;
pub use premultiply::*;

mod look;
use look::*;

mod sync;
pub use sync::*;


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
mod tests;
