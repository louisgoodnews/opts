//! The images of the text and script textures a mesh's material names: made once per index and
//! registered as the components the text and script systems draw into.

use super::*;

/// The image of a text texture of the model, made once per index and registered as a component
/// the text system draws into.
pub(super) fn text_image_for(
    (job, commands, images): (&WorldLoadJob, &mut Commands, &mut Assets<Image>),
    (mesh_def, owner, root): (&ModelMesh, Entity, Entity),
    (text_images, (i, clamp)): (&mut super::TextImageMap, (usize, bool)),
) -> Option<Handle<Image>> {
    let text_defs = &job.vehicle_text_defs;
    if let Some(h) = text_images.get(&(i, clamp)) {
        return Some(h.clone());
    }
    let Some(def) = text_defs.get(i) else {
        tracing::error!(at = %opts_bevy::position_tag(), mesh = %mesh_def.path, index = i, "[useTextTexture] names a text texture the model does not declare");
        return None;
    };
    let (w, h) = (def.width.max(1), def.height.max(1));
    let handle = images.add(opts_bevy::text_texture::text_image_addressed(
        w,
        h,
        vec![0; (w * h * 4) as usize],
        clamp,
    ));
    commands.spawn((
        VehicleTextTexture::new(owner, def, handle.clone()),
        Name::new(format!("text_texture_{i}_{}", def.variable)),
        ChildOf(root),
    ));
    text_images.insert((i, clamp), handle.clone());
    Some(handle)
}

/// The image of a script texture (a canvas the vehicle's scripts draw), made once per index.
pub(super) fn script_image_for(
    (job, commands, images): (&WorldLoadJob, &mut Commands, &mut Assets<Image>),
    (mesh_def, m, owner, root): (
        &ModelMesh,
        &opts_core::formats::vehicles::model::ModelMaterial,
        Entity,
        Entity,
    ),
    (tex_dirs, script_images, i): (&[PathBuf], &mut super::ImageMap, usize),
) -> Option<Handle<Image>> {
    let script_defs = &job.vehicle_script_defs;
    // Keyed by clamp too: a material with `[matl_texadress_clamp]` gets its own canvas image.
    let clamp = m.clamp_addressing;
    if let Some(h) = script_images.get(&(i, clamp)) {
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
    tracing::debug!(mesh = %mesh_def.path, index = i, clamp, tint = tint.is_some(), "script texture image made");
    let handle = images.add(opts_bevy::text_texture::text_image_addressed(
        w, h, blank, clamp,
    ));
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
    script_images.insert((i, clamp), handle.clone());
    Some(handle)
}
