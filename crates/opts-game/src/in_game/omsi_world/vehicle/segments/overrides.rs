//! Texture and depth-bias overrides for vehicle segment meshes.

use std::path::PathBuf;

use bevy::prelude::*;
use opts_bevy::TextureOverride;
use opts_core::formats::vehicles::model::ModelMesh;

use crate::in_game::omsi_world::vehicle_script_texture::{bright_colour, VehicleScriptTexture};
use crate::in_game::omsi_world::vehicle_text::VehicleTextTexture;
use crate::in_game::omsi_world::WorldLoadJob;

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

/// The image of a text texture of the model, made once per index and registered as a component
/// the text system draws into.
fn text_image_for(
    (job, commands, images): (&WorldLoadJob, &mut Commands, &mut Assets<Image>),
    (mesh_def, owner, root): (&ModelMesh, Entity, Entity),
    (text_images, i): (&mut std::collections::HashMap<usize, Handle<Image>>, usize),
) -> Option<Handle<Image>> {
    let text_defs = &job.vehicle_text_defs;
    if let Some(h) = text_images.get(&i) {
        return Some(h.clone());
    }
    let Some(def) = text_defs.get(i) else {
        tracing::error!(at = %opts_bevy::position_tag(), mesh = %mesh_def.path, index = i, "[useTextTexture] names a text texture the model does not declare");
        return None;
    };
    let (w, h) = (def.width.max(1), def.height.max(1));
    let handle = images.add(opts_bevy::text_texture::text_image(
        w,
        h,
        vec![0; (w * h * 4) as usize],
    ));
    commands.spawn((
        VehicleTextTexture::new(owner, def, handle.clone()),
        Name::new(format!("text_texture_{i}_{}", def.variable)),
        ChildOf(root),
    ));
    text_images.insert(i, handle.clone());
    Some(handle)
}

/// The image of a script texture (a canvas the vehicle's scripts draw), made once per index.
fn script_image_for(
    (job, commands, images): (&WorldLoadJob, &mut Commands, &mut Assets<Image>),
    (mesh_def, m, owner, root): (
        &ModelMesh,
        &opts_core::formats::vehicles::model::ModelMaterial,
        Entity,
        Entity,
    ),
    (tex_dirs, script_images, i): (
        &[PathBuf],
        &mut std::collections::HashMap<usize, Handle<Image>>,
        usize,
    ),
) -> Option<Handle<Image>> {
    let script_defs = &job.vehicle_script_defs;
    if let Some(h) = script_images.get(&i) {
        return Some(h.clone());
    }
    let Some(def) = script_defs.get(i) else {
        tracing::error!(at = %opts_bevy::position_tag(), mesh = %mesh_def.path, index = i, "[useScriptTexture] names a script texture the model does not declare");
        return None;
    };
    let (w, h) = (def.width.max(1), def.height.max(1));
    // A canvas used as `[matl_transmap] \S:n` is only the alpha: the colour
    // is the material's own texture (an LED matrix's `.dds`).
    let transmap = m
        .trans_map
        .as_deref()
        .is_some_and(|t| t.trim().to_ascii_lowercase().starts_with("\\s:"));
    let tint: Option<[u8; 3]> = transmap
        .then(|| {
            tex_dirs
                .iter()
                .find_map(|d| job.install.find_in(d, &m.texture))
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| opts_bevy::texture::load_bevy_image(&b).ok())
                .and_then(|img| bright_colour(&img))
        })
        .flatten();
    let mut blank = vec![0; (w * h * 4) as usize];
    if let Some(rgb) = tint {
        for p in blank.chunks_exact_mut(4) {
            p[..3].copy_from_slice(&rgb);
        }
    }
    let handle = images.add(opts_bevy::text_texture::text_image(w, h, blank));
    let mut component =
        VehicleScriptTexture::new(i, job.vehicle_script_textures.clone(), handle.clone());
    if let Some(rgb) = tint {
        component = component.with_tint(rgb);
    }
    // The colour texture may be named by a string variable
    // (`[matl_freetex]`: the LED matrix's `Matrix_Farbe`, the ATRON AFR4's
    // `AFR4_ST_texture` over a black placeholder, which has no colour of its
    // own to start from).
    if let (true, Some(free)) = (transmap, &m.free_tex) {
        component = component.following(owner, free.variable.trim(), tex_dirs.to_vec());
    }
    commands.spawn((
        component,
        Name::new(format!("script_texture_{i}")),
        ChildOf(root),
    ));
    script_images.insert(i, handle.clone());
    Some(handle)
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
