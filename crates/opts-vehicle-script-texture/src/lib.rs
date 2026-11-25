//! Dynamic script textures of vehicles (`[scripttexture]` + `[useScriptTexture]`).
//!
//! Scripts draw onto in-memory canvases using `(M.V.ST*)` callbacks.
//! When a script unlocks the texture (`STUnlock`), the updated canvas is uploaded
//! into the Bevy GPU `Image` used by the model meshes.

use bevy::prelude::*;
use opts_core::formats::script_texture::CpuScriptTexture;
use std::sync::{Arc, RwLock};

mod colour;
mod update;

pub use colour::{bright_colour, ColourMap, ColourSource};
pub use update::update_vehicle_script_textures;

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

#[cfg(test)]
mod tests;
