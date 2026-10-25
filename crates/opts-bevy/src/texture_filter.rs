//! The shared sampler of OMSI object textures and its user setting ("Texture filtering").
//!
//! Bevy's default sampler is *nearest* filtering with anisotropy 1, which makes textures blocky
//! when magnified and noisy when minified. Every texture the loader creates (object textures,
//! hot-reloaded textures, text textures) gets the sampler of [`object_sampler`]: repeat
//! addressing, linear magnification/minification, linear mipmap filtering and an anisotropy that
//! follows [`TextureFiltering`]. The terrain shader builds its own array texture and is not
//! affected.
//!
//! The current choice is process-wide ([`set_texture_filtering`]): textures are created on
//! loader threads that have no access to the ECS world. A change applies to textures loaded
//! afterwards (and to hot-reloaded ones).

use std::sync::atomic::{AtomicU8, Ordering};

use bevy::image::{
    ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerBorderColor,
    ImageSamplerDescriptor,
};
use serde::{Deserialize, Serialize};

/// Quality of texture filtering (the "Texture filtering" setting).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TextureFiltering {
    /// Nearest-neighbour, no mipmaps: the old, blocky look (and the fastest).
    Off,
    /// Trilinear with 4x anisotropic filtering.
    X4,
    /// Trilinear with 8x anisotropic filtering.
    X8,
    /// Trilinear with 16x anisotropic filtering (default).
    #[default]
    X16,
}

impl TextureFiltering {
    /// All choices in menu order.
    pub const ALL: [Self; 4] = [Self::Off, Self::X4, Self::X8, Self::X16];

    /// The anisotropy clamp (1 = off).
    #[must_use]
    pub const fn anisotropy(self) -> u16 {
        match self {
            Self::Off => 1,
            Self::X4 => 4,
            Self::X8 => 8,
            Self::X16 => 16,
        }
    }

    /// Whether textures get mipmaps and linear filtering at all.
    #[must_use]
    pub const fn is_filtered(self) -> bool {
        !matches!(self, Self::Off)
    }

    /// The stable key used in settings, the control protocol and on the command line.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::X4 => "4",
            Self::X8 => "8",
            Self::X16 => "16",
        }
    }

    /// The label shown in the settings screen.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::X4 => "4x",
            Self::X8 => "8x",
            Self::X16 => "16x (default)",
        }
    }

    /// Parses a [`key`](Self::key) (also `4x`, case-insensitive).
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let t = text.trim().to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|f| f.key() == t || format!("{}x", f.key()) == t)
    }

    fn to_u8(self) -> u8 {
        self.anisotropy() as u8
    }

    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Off,
            4 => Self::X4,
            8 => Self::X8,
            _ => Self::X16,
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(16);

/// Sets the filtering used for textures created from now on.
pub fn set_texture_filtering(filtering: TextureFiltering) {
    CURRENT.store(filtering.to_u8(), Ordering::Relaxed);
}

/// The filtering used for textures created now.
#[must_use]
pub fn texture_filtering() -> TextureFiltering {
    TextureFiltering::from_u8(CURRENT.load(Ordering::Relaxed))
}

/// The sampler descriptor of OMSI object textures for `filtering`; every field is stated, nothing
/// is left to Bevy's defaults.
#[must_use]
pub fn object_sampler_descriptor(filtering: TextureFiltering) -> ImageSamplerDescriptor {
    let filter = if filtering.is_filtered() {
        ImageFilterMode::Linear
    } else {
        ImageFilterMode::Nearest
    };
    ImageSamplerDescriptor {
        label: None,
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        mag_filter: filter,
        min_filter: filter,
        mipmap_filter: filter,
        lod_min_clamp: 0.0,
        lod_max_clamp: 32.0,
        compare: None,
        anisotropy_clamp: filtering.anisotropy(),
        border_color: None::<ImageSamplerBorderColor>,
    }
}

/// The sampler of OMSI object textures for `filtering`.
#[must_use]
pub fn object_sampler(filtering: TextureFiltering) -> ImageSampler {
    ImageSampler::Descriptor(object_sampler_descriptor(filtering))
}

/// The sampler for a texture created now (uses [`texture_filtering`]).
#[must_use]
pub fn current_object_sampler() -> ImageSampler {
    object_sampler(texture_filtering())
}

/// Finishes a freshly decoded object texture: the shared sampler and, unless filtering is *Off*,
/// a generated mip chain for uncompressed images that lack one (hard cutouts get a
/// coverage-preserving alpha so foliage does not thin out with distance). Block-compressed
/// textures keep what the file has.
pub fn prepare_object_texture(image: &mut bevy::image::Image) {
    prepare(image, false);
}

/// Like [`prepare_object_texture`] for rendered text (transparent background): colours are
/// averaged weighted by alpha so distant text keeps its colour.
pub fn prepare_text_texture(image: &mut bevy::image::Image) {
    prepare(image, true);
}

/// For a canvas of small bright dots (LED matrices): wrapping like every OMSI texture, linear
/// sampling and no mip chain, which would average the dots into the dark between them.
pub fn prepare_led_texture(image: &mut bevy::image::Image) {
    use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..ImageSamplerDescriptor::default()
    });
}

fn prepare(image: &mut bevy::image::Image, weight_colour_by_alpha: bool) {
    let filtering = texture_filtering();
    image.sampler = object_sampler(filtering);
    if !filtering.is_filtered() {
        return;
    }
    dev_dxt::visit(image);
    let cutout = !weight_colour_by_alpha
        && image
            .data
            .as_deref()
            .is_some_and(crate::texture::looks_like_cutout);
    crate::texture::generate_mip_chain(
        image,
        crate::texture::MipOptions {
            alpha_cutoff: cutout.then_some(0.5),
            weight_colour_by_alpha,
        },
    );
}

/// Developer measurement for M-0058 phase A (`OPTS_DEV_DXT_MIPS`): block-compressed textures
/// without a mip chain are counted (`count`), or decoded to RGBA8 so they get a generated chain
/// like every uncompressed texture (`rgba`, option 1 of the milestone). Unset: nothing happens.
pub mod dev_dxt {
    use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
    use std::sync::OnceLock;

    use bevy::render::render_resource::TextureFormat;

    static TEXTURES: AtomicU64 = AtomicU64::new(0);
    static PIXELS: AtomicU64 = AtomicU64::new(0);
    static DECODE_MICROS: AtomicU64 = AtomicU64::new(0);

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Mode {
        Off,
        Count,
        Rgba,
    }

    fn mode() -> Mode {
        static MODE: OnceLock<Mode> = OnceLock::new();
        *MODE.get_or_init(|| match std::env::var("OPTS_DEV_DXT_MIPS").as_deref() {
            Ok("count") => Mode::Count,
            Ok("rgba") => Mode::Rgba,
            _ => Mode::Off,
        })
    }

    /// Counts and, in `rgba` mode, decodes a chainless BC1/2/3 texture in place.
    pub(super) fn visit(image: &mut bevy::image::Image) {
        let mode = mode();
        let desc = &image.texture_descriptor;
        if mode == Mode::Off
            || desc.mip_level_count != 1
            || !matches!(
                desc.format,
                TextureFormat::Bc1RgbaUnormSrgb
                    | TextureFormat::Bc2RgbaUnormSrgb
                    | TextureFormat::Bc3RgbaUnormSrgb
            )
        {
            return;
        }
        TEXTURES.fetch_add(1, Relaxed);
        PIXELS.fetch_add(
            u64::from(desc.size.width) * u64::from(desc.size.height),
            Relaxed,
        );
        if mode == Mode::Rgba {
            let started = std::time::Instant::now();
            if let Some(decoded) = crate::texture::decode_to_rgba8(image) {
                let sampler = image.sampler.clone();
                *image = decoded;
                image.sampler = sampler;
            }
            DECODE_MICROS.fetch_add(started.elapsed().as_micros() as u64, Relaxed);
        }
    }

    /// `(textures, base pixels, decode microseconds)` seen so far in this process.
    #[must_use]
    pub fn stats() -> (u64, u64, u64) {
        (
            TEXTURES.load(Relaxed),
            PIXELS.load(Relaxed),
            DECODE_MICROS.load(Relaxed),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_16x_trilinear() {
        let d = object_sampler_descriptor(TextureFiltering::default());
        assert_eq!(d.anisotropy_clamp, 16);
        assert_eq!(d.mag_filter, ImageFilterMode::Linear);
        assert_eq!(d.min_filter, ImageFilterMode::Linear);
        assert_eq!(d.mipmap_filter, ImageFilterMode::Linear);
        assert_eq!(d.address_mode_u, ImageAddressMode::Repeat);
        assert_eq!(d.address_mode_v, ImageAddressMode::Repeat);
        assert_eq!(d.address_mode_w, ImageAddressMode::Repeat);
    }

    #[test]
    fn off_is_nearest_without_anisotropy() {
        let d = object_sampler_descriptor(TextureFiltering::Off);
        assert_eq!(d.anisotropy_clamp, 1);
        assert_eq!(d.mag_filter, ImageFilterMode::Nearest);
        assert_eq!(d.mipmap_filter, ImageFilterMode::Nearest);
    }

    #[test]
    fn anisotropy_levels_and_keys_round_trip() {
        for f in TextureFiltering::ALL {
            assert_eq!(TextureFiltering::parse(f.key()), Some(f));
            assert_eq!(TextureFiltering::from_u8(f.to_u8()), f);
            if f.is_filtered() {
                // wgpu requires all filters to be linear when anisotropy is above 1.
                let d = object_sampler_descriptor(f);
                assert_eq!(d.anisotropy_clamp, f.anisotropy());
                assert_eq!(d.min_filter, ImageFilterMode::Linear);
            }
        }
        assert_eq!(TextureFiltering::parse("8X"), Some(TextureFiltering::X8));
        assert_eq!(TextureFiltering::parse("2"), None);
    }

    #[test]
    fn current_setting_is_used_by_the_shared_sampler() {
        // One test mutates the global; other tests only use the pure functions.
        set_texture_filtering(TextureFiltering::X4);
        let ImageSampler::Descriptor(d) = current_object_sampler() else {
            panic!("expected a descriptor");
        };
        assert_eq!(d.anisotropy_clamp, 4);
        set_texture_filtering(TextureFiltering::X16);
    }
}
