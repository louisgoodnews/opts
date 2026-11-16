//! Texture and depth-bias overrides for vehicle segment meshes.

use std::path::PathBuf;

use bevy::prelude::*;
use opts_bevy::TextureOverride;
use opts_core::formats::vehicles::model::ModelMesh;

use crate::in_game::omsi_world::vehicle_script_texture::{bright_colour, VehicleScriptTexture};
use crate::in_game::omsi_world::vehicle_text::VehicleTextTexture;
use crate::in_game::omsi_world::WorldLoadJob;

mod images;
use images::{script_image_for, text_image_for};

/// Depth bias per earlier mesh of the same file name (constant depth-bias units).
const COPY_DEPTH_BIAS: f32 = 1.0;

/// The file name of a `[mesh]` path without folders and extension, lower case: layer copies of
/// one display sit in different folders under one name.
pub(crate) fn mesh_stem(path: &str) -> String {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    name.rsplit_once('.')
        .map_or(name, |(stem, _)| stem)
        .to_lowercase()
}

/// The parts of the `copy`-th mesh of one file name in the model (0: the first). OMSI draws a
/// model's meshes in file order and a later surface at equal depth wins, which displays rely on:
/// the Lion's City's VDV display stacks a background (`VDV\BHD_LC_VDV_Display.o3d`, the
/// placeholder texture), the page and its overlays (`VDV\Display\BHD_LC_VDV_Display.o3d`) on
/// one screen. Bevy keeps no such order, so a later copy gets a slightly larger depth bias (its
/// own material, shared through the pool).
pub(crate) fn in_file_order(
    job: &mut WorldLoadJob,
    materials: &mut Assets<StandardMaterial>,
    mut parts: Vec<opts_bevy::ModelPart>,
    copy: u32,
) -> Vec<opts_bevy::ModelPart> {
    if copy == 0 {
        return parts;
    }
    for part in &mut parts {
        let Some(mut m) = materials.get(&part.material).cloned() else {
            continue;
        };
        m.depth_bias += COPY_DEPTH_BIAS * copy as f32;
        part.material = job.cache.pool.material(materials, m);
    }
    parts
}


/// Builds the texture overrides of `mesh_def`.
/// Images by material index.
pub(crate) type ImageMap = std::collections::HashMap<usize, Handle<Image>>;

pub(crate) fn build_overrides(
    (job, commands, images): (&WorldLoadJob, &mut Commands, &mut Assets<Image>),
    (mesh_def, owner, root): (&ModelMesh, Entity, Entity),
    tex_dirs: &[PathBuf],
    (text_images, script_images): (&mut ImageMap, &mut ImageMap),
) -> Vec<TextureOverride> {
    mesh_def
        .materials
        .iter()
        .map(|m| TextureOverride {
            slot: m.slot_index as usize,
            texture: m.texture.clone(),
            alpha: m.alpha_mode,
            alpha_variable: m.alpha_scale_var.clone(),
            trans_map: m.trans_map.clone().filter(|t| !t.trim().is_empty()),
            image: m
                .text_texture_index
                .and_then(|i| {
                    text_image_for(
                        (job, commands, images),
                        (mesh_def, owner, root),
                        (&mut *text_images, i),
                    )
                })
                .or_else(|| {
                    m.script_texture_index.and_then(|i| {
                        script_image_for(
                            (job, commands, images),
                            (mesh_def, m, owner, root),
                            (tex_dirs, &mut *script_images, i),
                        )
                    })
                }),
            light_map: m.light_map.clone(),
            night_map: m.night_map.clone().filter(|t| !t.trim().is_empty()),
            // A texture named by a string variable (`[matl_freetex]`): the C2 display's
            // pages. Canvases follow theirs through the tint above.
            free_texture: m
                .free_tex
                .as_ref()
                .filter(|_| m.script_texture_index.is_none())
                .map(|f| f.variable.trim().to_owned())
                .filter(|v| !v.is_empty()),
        })
        .collect()
}
