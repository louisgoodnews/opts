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
        }
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
                        img.data = Some(canvas.raw_rgba().to_vec());
                    }
                    canvas.clear_dirty();
                }
            }
        }
    }
}
