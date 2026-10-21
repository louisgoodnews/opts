//! Dynamic script textures of vehicles (`[scripttexture]` + `[useScriptTexture]`).
//!
//! Scripts draw onto in-memory canvases using `(M.V.ST*)` callbacks.
//! When a script unlocks the texture (`STUnlock`), the updated canvas is uploaded
//! into the Bevy GPU `Image` used by the model meshes.

use bevy::prelude::*;
use opts_core::formats::script_texture::{CpuScriptTexture, ScriptTextureCanvas};
use std::sync::{Arc, RwLock};

/// One script texture of a vehicle model.
#[derive(Component)]
pub struct VehicleScriptTexture {
    /// Slot index of the script texture (`0..N`).
    pub index: usize,
    /// Shared CPU canvases of the vehicle.
    pub canvases: Arc<RwLock<Vec<CpuScriptTexture>>>,
    /// GPU Image handle bound to model meshes.
    pub image: Handle<Image>,
    /// The colour of a canvas that serves as the transparency map of a material
    /// (`[matl_transmap] \S:n`, the LED matrices): the material's own texture gives the colour,
    /// the canvas only its alpha. `None`: the canvas is the texture.
    pub tint: Option<[u8; 3]>,
}

impl VehicleScriptTexture {
    /// Creates a new script texture component tracking slot `index`.
    #[must_use]
    pub fn new(
        index: usize,
        canvases: Arc<RwLock<Vec<CpuScriptTexture>>>,
        image: Handle<Image>,
    ) -> Self {
        Self {
            index,
            canvases,
            image,
            tint: None,
        }
    }

    /// The canvas as a transparency map over the colour `rgb`.
    #[must_use]
    pub fn with_tint(mut self, rgb: [u8; 3]) -> Self {
        self.tint = Some(rgb);
        self
    }
}

/// Uploads unlocked/dirty script texture canvases into Bevy GPU images.
pub fn update_vehicle_script_textures(
    mut images: ResMut<Assets<Image>>,
    targets: Query<&VehicleScriptTexture>,
) {
    for target in &targets {
        if let Ok(mut canvases) = target.canvases.write() {
            if let Some(canvas) = canvases.get_mut(target.index) {
                if canvas.is_dirty() && !canvas.is_locked() {
                    if let Some(mut img) = images.get_mut(&target.image) {
                        let mut data = canvas.raw_rgba().to_vec();
                        if let Some(rgb) = target.tint {
                            for p in data.chunks_exact_mut(4) {
                                p[..3].copy_from_slice(&rgb);
                            }
                        }
                        // A new image with its mip chain: setting the data alone leaves the
                        // image's other mip levels (made for the blank one) empty.
                        *img = opts_bevy::text_texture::text_image(
                            canvas.width(),
                            canvas.height(),
                            data,
                        );
                    }
                    canvas.clear_dirty();
                }
            }
        }
    }
}

/// The colour of a light-emitting texture (an LED's `.dds`): the average of its bright pixels.
#[must_use]
pub fn bright_colour(image: &Image) -> Option<[u8; 3]> {
    let rgba = opts_bevy::texture::decode_to_rgba8(image)?;
    let data = rgba.data.as_deref()?;
    let (mut sum, mut n) = ([0u64; 3], 0u64);
    for p in data
        .chunks_exact(4)
        .take(rgba.width() as usize * rgba.height() as usize)
    {
        if p[..3].iter().copied().max().unwrap_or(0) > 96 {
            for i in 0..3 {
                sum[i] += u64::from(p[i]);
            }
            n += 1;
        }
    }
    (n > 0).then(|| [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8])
}
