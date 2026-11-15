//! The process-wide counters of the generated mip chains (`sim.get` shows them).

use super::*;

/// Running totals of the chains generated in this process (for load-time reports).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MipStats {
    /// Textures that received a generated chain.
    pub textures: u64,
    /// Time spent generating, in microseconds.
    pub micros: u64,
}

pub(super) static GENERATED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub(super) static MICROS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The totals so far.
#[must_use]
pub fn mip_stats() -> MipStats {
    use std::sync::atomic::Ordering::Relaxed;
    MipStats {
        textures: GENERATED.load(Relaxed),
        micros: MICROS.load(Relaxed),
    }
}

/// Gives an uncompressed `Rgba8UnormSrgb` image a full mip chain. Returns `true` when levels were
/// added; images that already have a chain, are block-compressed, masks (`R8`) or have
/// inconsistent data are left untouched (`false`).
pub fn generate_mip_chain(image: &mut Image, options: MipOptions) -> bool {
    let desc = &image.texture_descriptor;
    if desc.mip_level_count > 1
        || desc.format != TextureFormat::Rgba8UnormSrgb
        || desc.size.depth_or_array_layers != 1
        || desc.size.width.max(desc.size.height) < 2
    {
        return false;
    }
    let (w, h) = (desc.size.width, desc.size.height);
    let started = std::time::Instant::now();
    let Some(chain) = image
        .data
        .as_deref()
        .and_then(|base| rgba8_srgb_chain(base, w, h, options))
    else {
        return false;
    };
    image.data = Some(chain);
    image.texture_descriptor.mip_level_count = full_chain_levels(w, h);
    use std::sync::atomic::Ordering::Relaxed;
    GENERATED.fetch_add(1, Relaxed);
    MICROS.fetch_add(started.elapsed().as_micros() as u64, Relaxed);
    true
}
