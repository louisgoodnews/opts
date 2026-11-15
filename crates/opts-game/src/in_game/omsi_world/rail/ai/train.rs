//! Builds the units of a train: meshes hidden until the train is placed.

use super::*;

/// The unit entities of a train spec with their meshes, hidden until placed; `None` (and
/// nothing left behind) while a mesh is still decoding in the background.
pub(in super::super) fn build_train(
    spec: &TrainSpec,
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    assets: &mut SpawnAssets,
    scripted: bool,
) -> Option<(Entity, VehicleComposition, Vec<ScriptCanvases>)> {
    let (meshes, materials, images) = (&mut *assets.0, &mut *assets.1, &mut *assets.2);
    let mut units = Vec::new();
    let mut canvas_sets: Vec<ScriptCanvases> = Vec::new();
    for u in &spec.units {
        let root = commands
            .spawn((
                Transform::default(),
                Visibility::Hidden,
                Name::new(u.name.clone()),
                InGameEntity,
                opts_bevy::AiVariables::default(),
            ))
            .id();
        // A scripted unit draws its display on its own canvases; the others show no text.
        if scripted {
            job.vehicle_text_defs = u.defs.text.clone();
            job.vehicle_script_defs = u.defs.script.clone();
            let canvases = u
                .defs
                .script
                .iter()
                .map(|st| {
                    opts_core::formats::script_texture::CpuScriptTexture::new(st.width, st.height)
                })
                .collect();
            job.vehicle_script_textures = std::sync::Arc::new(std::sync::RwLock::new(canvases));
            canvas_sets.push(job.vehicle_script_textures.clone());
        } else {
            job.vehicle_text_defs.clear();
        }
        let (count, ..) = load_segment_meshes(
            job,
            commands,
            (&mut *meshes, &mut *materials, &mut *images),
            root,
            &u.lod,
            &u.model_dir,
            &u.tex_dirs,
        );
        units.push((root, u.behind, u.reversed, u.span, count));
    }
    if units.iter().any(|u| u.4 == 0) {
        // Meshes still decoding in the background: discard and try again on the next tick.
        for (e, ..) in &units {
            commands.entity(*e).despawn();
        }
        return None;
    }
    let head = units[0].0;
    let part = |i: usize, u: &(Entity, f32, bool, f32, usize)| Part {
        entity: u.0,
        name: spec.units[i].name.clone(),
        behind: u.1,
        half_span: u.3,
        reversed: u.2,
        meshes: u.4,
        vertices: 0,
        body: spec.units[i].body,
    };
    let composition = VehicleComposition {
        head: part(0, &units[0]),
        followers: units
            .iter()
            .enumerate()
            .skip(1)
            .map(|(i, u)| part(i, u))
            .collect(),
    };
    Some((head, composition, canvas_sets))
}
