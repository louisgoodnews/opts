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
    /// A colour texture of the canvas's proportions (a screen's picture, the ATRON AFR4's
    /// `ST_2.bmp`): each pixel takes its colour from it instead of the [`Self::tint`].
    pub colour_map: Option<Arc<ColourMap>>,
    /// The string variable that names the colour texture (`[matl_freetex]`) and where to look
    /// for the file; the tint follows it.
    pub colour_source: Option<ColourSource>,
}

/// A decoded colour texture (RGBA8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColourMap {
    /// Width (px).
    pub width: u32,
    /// Height (px).
    pub height: u32,
    /// Pixels, row by row.
    pub rgba: Vec<u8>,
}

impl ColourMap {
    /// Whether the map covers a canvas of `width × height` as a picture of the same proportions
    /// (not a small swatch, such as an LED's texture).
    #[must_use]
    pub fn fits(&self, width: u32, height: u32) -> bool {
        self.width * 2 >= width
            && self.height * 2 >= height
            && (u64::from(self.width) * u64::from(height))
                .abs_diff(u64::from(self.height) * u64::from(width))
                <= u64::from(width.max(height))
    }

    /// Puts the map's colour under every pixel of a canvas of `width × height` (nearest pixel),
    /// keeping the canvas's alpha.
    pub fn paint(&self, data: &mut [u8], width: u32, height: u32) {
        for (i, p) in data.chunks_exact_mut(4).enumerate() {
            let (x, y) = (i as u32 % width, i as u32 / width);
            let (mx, my) = (x * self.width / width, y * self.height / height);
            let o = ((my * self.width + mx) * 4) as usize;
            if let Some(c) = self.rgba.get(o..o + 3) {
                p[..3].copy_from_slice(c);
            }
        }
    }
}

/// Where a tinted canvas takes its colour texture's name from.
#[derive(Debug, Clone)]
pub struct ColourSource {
    /// The entity whose engine owns the variable.
    pub owner: Entity,
    /// The string variable.
    pub variable: String,
    /// The folders to look for the texture in.
    pub dirs: Vec<std::path::PathBuf>,
    /// The name last looked at.
    pub last: String,
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
            colour_map: None,
            colour_source: None,
        }
    }

    /// The tint follows the colour texture the string variable `variable` of `owner` names.
    #[must_use]
    pub fn following(
        mut self,
        owner: Entity,
        variable: &str,
        dirs: Vec<std::path::PathBuf>,
    ) -> Self {
        self.colour_source = Some(ColourSource {
            owner,
            variable: variable.to_owned(),
            dirs,
            last: String::new(),
        });
        self
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
    mut redrawn: ResMut<opts_bevy::text_texture::RedrawnImages>,
    mut targets: Query<&mut VehicleScriptTexture>,
    scripts: Query<&opts_bevy::VehicleScripts>,
) {
    for mut target in &mut targets {
        // A colour texture named by a string variable: when the name changes, the tint changes
        // and the image is made again.
        let mut retint = false;
        if let Some(source) = target.colour_source.as_mut() {
            let name = scripts
                .get(source.owner)
                .ok()
                .and_then(|s| {
                    s.engine
                        .lock()
                        .ok()
                        .map(|e| e.get_string(&source.variable).to_owned())
                })
                .unwrap_or_default();
            if !name.is_empty() && name != source.last {
                source.last.clone_from(&name);
                let image = source
                    .dirs
                    .iter()
                    .find_map(|d| opts_bevy::script_text::resolve_path(d, &name))
                    .and_then(|p| std::fs::read(p).ok())
                    .and_then(|b| opts_bevy::texture::load_bevy_image(&b).ok());
                if let Some(rgb) = image.as_ref().and_then(bright_colour) {
                    target.tint = Some(rgb);
                    retint = true;
                }
                target.colour_map = image.as_ref().and_then(colour_map).map(Arc::new);
            }
        }
        if let Ok(mut canvases) = target.canvases.write() {
            if let Some(canvas) = canvases.get_mut(target.index) {
                if retint {
                    canvas.mark_dirty();
                }
                if canvas.is_dirty() && !canvas.is_locked() {
                    if let Some(mut img) = images.get_mut(&target.image) {
                        // Either picture may get a new GPU texture (see `text_image`).
                        redrawn.0.insert(target.image.id());
                        let mut data = canvas.raw_rgba().to_vec();
                        let (w, h) = (canvas.width(), canvas.height());
                        if let Some(map) = target.colour_map.as_ref().filter(|m| m.fits(w, h)) {
                            // A screen: the picture's colours under the canvas's alpha.
                            map.paint(&mut data, w, h);
                            *img = opts_bevy::text_texture::text_image(w, h, data);
                            canvas.clear_dirty();
                            continue;
                        }
                        if let Some(rgb) = target.tint {
                            for p in data.chunks_exact_mut(4) {
                                p[..3].copy_from_slice(&rgb);
                            }
                        }
                        // A new image with its mip chain: setting the data alone leaves the
                        // image's other mip levels (made for the blank one) empty.
                        if target.tint.is_some() {
                            // An LED canvas is a grid of small bright dots: a mip chain would
                            // average them into the dark between them and the board would fade
                            // out with distance. Linear sampling of the full size keeps them lit.
                            *img = opts_bevy::text_texture::led_image(
                                canvas.width(),
                                canvas.height(),
                                data,
                            );
                            canvas.clear_dirty();
                            continue;
                        }
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

/// A colour texture's pixels.
fn colour_map(image: &Image) -> Option<ColourMap> {
    let rgba = opts_bevy::texture::decode_to_rgba8(image)?;
    let (width, height) = (rgba.width(), rgba.height());
    let data = rgba.data?;
    Some(ColourMap {
        width,
        height,
        rgba: data,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_of_the_canvas_proportions_colours_each_pixel_a_swatch_does_not() {
        // 2 x 1: red left, blue right.
        let map = ColourMap {
            width: 2,
            height: 1,
            rgba: vec![255, 0, 0, 255, 0, 0, 255, 255],
        };
        assert!(
            map.fits(4, 2) && !map.fits(8, 4),
            "at least half the canvas size"
        );
        assert!(!map.fits(4, 4), "other proportions: a swatch");
        let mut data = vec![0, 0, 0, 200, 0, 0, 0, 0, 0, 0, 0, 50, 0, 0, 0, 255];
        map.paint(&mut data, 4, 1);
        assert_eq!(&data[0..4], &[255, 0, 0, 200], "left: red, alpha kept");
        assert_eq!(&data[12..16], &[0, 0, 255, 255], "right: blue");
    }
}
