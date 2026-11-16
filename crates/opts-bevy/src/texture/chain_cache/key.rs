//! The identity of one cached chain: a hash of its texture, the size and the generator version.

use super::*;

/// Identity of one cached chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainKey {
    pub(super) hash: String,
    pub(super) format: u32,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) generator: u32,
}

impl ChainKey {
    /// The key of a texture: its format, size and level-0 bytes, and the generator version.
    #[must_use]
    pub fn new(
        format: TextureFormat,
        width: u32,
        height: u32,
        level0: &[u8],
        generator: u32,
    ) -> Self {
        let format = format_code(format);
        let mut hasher = Sha256::new();
        hasher.update(format.to_le_bytes());
        hasher.update(width.to_le_bytes());
        hasher.update(height.to_le_bytes());
        hasher.update(level0);
        let hash = hasher
            .finalize()
            .iter()
            .take(16)
            .map(|b| format!("{b:02x}"))
            .collect();
        Self {
            hash,
            format,
            width,
            height,
            generator,
        }
    }

    pub(super) fn file_name(&self) -> String {
        format!("{}-g{}.mip", self.hash, self.generator)
    }

    pub(super) fn block_bytes(&self) -> usize {
        if self.format == 1 {
            8
        } else {
            16
        }
    }
}

fn format_code(format: TextureFormat) -> u32 {
    match format {
        TextureFormat::Bc1RgbaUnormSrgb | TextureFormat::Bc1RgbaUnorm => 1,
        TextureFormat::Bc2RgbaUnormSrgb | TextureFormat::Bc2RgbaUnorm => 2,
        TextureFormat::Bc3RgbaUnormSrgb | TextureFormat::Bc3RgbaUnorm => 3,
        _ => 0,
    }
}
