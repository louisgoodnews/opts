//! Dynamic text textures of vehicles (`[texttexture]` / `[texttexture_enh]` + `[useTextTexture]`).
//!
//! The scripts write the displayed text into a string variable (`(S.$.Mtx_Linie)`); each text
//! texture of the model draws that variable with an OMSI bitmap font into an image the meshes use
//! instead of their (non-existent) placeholder texture. The image is redrawn when the text
//! changes.

use bevy::prelude::*;
use opts_bevy::text_texture::TextSpec;
use opts_bevy::VehicleScripts;
use opts_core::formats::vehicles::model::TextTexture;

use super::WorldLoadJob;

/// Redraws at most this many text textures per frame (rendering is cheap but a freshly started
/// vehicle has dozens).
const MAX_REDRAWS_PER_FRAME: usize = 8;

/// One text texture of a vehicle model.
#[derive(Component)]
pub struct VehicleTextTexture {
    owner: Entity,
    variable: String,
    spec: TextSpec,
    image: Handle<Image>,
    /// The text currently drawn (`None` before the first draw).
    shown: Option<String>,
}

impl VehicleTextTexture {
    /// A text texture showing the string variable `def.variable` of `owner`'s scripts.
    #[must_use]
    pub fn new(owner: Entity, def: &TextTexture, image: Handle<Image>) -> Self {
        Self {
            owner,
            variable: def.variable.clone(),
            spec: TextSpec {
                font: def.font_name.clone(),
                text: String::new(),
                width: def.width.max(1),
                height: def.height.max(1),
                mono: !def.full_color,
                rgb: def.color_rgb,
                orientation: def.orientation.unwrap_or(0),
            },
            image,
            shown: None,
        }
    }
}

/// Draws the string variables of the vehicles' scripts into their text textures.
pub fn update_vehicle_text_textures(
    job: Option<ResMut<WorldLoadJob>>,
    mut images: ResMut<Assets<Image>>,
    scripts: Query<&VehicleScripts>,
    mut targets: Query<&mut VehicleTextTexture>,
) {
    let Some(mut job) = job else { return };
    let job = &mut *job;
    let mut redraws = 0;
    for mut t in &mut targets {
        let Ok(vehicle) = scripts.get(t.owner) else {
            continue;
        };
        let Ok(engine) = vehicle.engine.lock() else {
            continue;
        };
        let text = engine.get_string(&t.variable).to_owned();
        drop(engine);
        if t.shown.as_deref() == Some(text.as_str()) {
            continue;
        }
        if redraws >= MAX_REDRAWS_PER_FRAME {
            break;
        }
        redraws += 1;
        t.spec.text.clone_from(&text);
        let data = if text.is_empty() {
            Some(vec![0u8; (t.spec.width * t.spec.height * 4) as usize])
        } else {
            job.fonts.render_data(&job.install, &t.spec)
        };
        if let (Some(data), Some(mut image)) = (data, images.get_mut(&t.image)) {
            image.data = Some(data);
        }
        t.shown = Some(text);
    }
}
