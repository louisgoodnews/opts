//! The per-frame upload of script texture canvases into the GPU images.

use std::sync::Arc;

use bevy::prelude::*;
use opts_core::formats::script_texture::ScriptTextureCanvas;

use super::colour::colour_map;
use super::{bright_colour, VehicleScriptTexture};

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
