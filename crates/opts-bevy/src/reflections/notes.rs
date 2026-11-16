//! Material notes recorded by the model loader, so the wet-surface pass can read each
//! material's original colour, roughness and alpha mode.

use std::collections::HashMap;
use std::sync::Mutex;

use bevy::pbr::StandardMaterial;
use bevy::prelude::*;

use super::flags::switches;

/// What the model loader recorded about one material, enough to treat it afterwards.
#[derive(Debug, Clone)]
pub struct MaterialNote {
    /// Texture file name of the material, lower case.
    pub texture: String,
    /// Original alpha mode.
    pub alpha: AlphaMode,
    /// Original base colour.
    pub base_color: Color,
    /// Original perceptual roughness.
    pub roughness: f32,
    /// The texture's property file has `[moisture]`: the material takes the wetness.
    pub moisture: bool,
    /// A vehicle's tyre (see [`super::vehicle_wet::is_tyre_texture`]): takes the tyre share.
    pub tyre: bool,
}

static NOTES: Mutex<Option<HashMap<AssetId<StandardMaterial>, MaterialNote>>> = Mutex::new(None);

/// Called by the model loader for every material it creates. Free unless a switch is on or the
/// material takes the wetness (`moisture`, from the texture's `[moisture]` flag, or `tyre`).
pub fn note_material(
    handle: &Handle<StandardMaterial>,
    texture: Option<&str>,
    (moisture, tyre): (bool, bool),
    m: &StandardMaterial,
) {
    if !moisture && !tyre && !switches().any() {
        return;
    }
    let note = MaterialNote {
        texture: texture.unwrap_or("").to_lowercase(),
        alpha: m.alpha_mode,
        base_color: m.base_color,
        roughness: m.perceptual_roughness,
        moisture,
        tyre,
    };
    if let Ok(mut g) = NOTES.lock() {
        g.get_or_insert_with(HashMap::new).insert(handle.id(), note);
    }
}

pub(super) fn note_of(id: AssetId<StandardMaterial>) -> Option<MaterialNote> {
    NOTES.lock().ok()?.as_ref()?.get(&id).cloned()
}
