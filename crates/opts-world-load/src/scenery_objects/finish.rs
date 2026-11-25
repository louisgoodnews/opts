//! Placing an object's parts: flat static scenery, or the runtime hierarchy of a scripted object.

use super::*;

impl WorldLoadJob {
    /// Whether an object is flat static scenery: no script, light, animation, visibility rule,
    /// LOD group or driven light map, and no tree that moves in the wind (M-0144), so it is only
    /// ever drawn at its placement (`needs_runtime` would be false and nothing changes its parts).
    pub(in crate) fn is_flat_static(
        sco: &SceneryObject,
        mesh_parts: &[Vec<ModelPart>],
        template: Option<&Arc<ScriptTemplate>>,
    ) -> bool {
        template.is_none_or(|t| !t.has_scripts)
            && sco.sound.is_none()
            && !super::super::trees::is_mesh_tree(sco)
            && sco.lights.is_empty()
            && sco.meshes.iter().all(|m| {
                m.lod_threshold.is_none() && m.visible.is_none() && m.animations.is_empty()
            })
            && mesh_parts
                .iter()
                .flatten()
                .all(|p| p.light_variable.is_none())
    }

    /// Hangs a placed object's parts under `root`: flat as before, or as a runtime hierarchy
    /// when the object has LOD groups, rules, lights, light maps or scripts.
    pub(in crate) fn finish_object(
        &mut self,
        commands: &mut Commands,
        (meshes, materials): (&mut Assets<Mesh>, &mut Assets<StandardMaterial>),
        root: Entity,
        (sco, obj): (&SceneryObject, &MapObjectInstance),
        mesh_parts: Vec<Vec<ModelPart>>,
        template: Option<Arc<ScriptTemplate>>,
    ) {
        self.scenery.dev_look.offer(&obj.sco_file, root);
        commands
            .entity(root)
            .insert((opts_bevy::SceneryDrawRoot, Visibility::Inherited));
        // `[nomaplighting]`: recorded on the root (version 1 has no behaviour yet).
        if sco.no_map_lighting {
            commands.entity(root).insert(opts_bevy::NoMapLighting);
        }
        // A tree modelled as a mesh: its foliage moves in the wind (M-0144 phase C).
        if super::super::trees::is_mesh_tree(sco) {
            commands
                .entity(root)
                .insert(opts_bevy::tree_wind::MeshTreeRoot);
        }
        // `[maplight]` ground lights: markers the map light pool hands its point lights to.
        opts_bevy::scenery_runtime::spawn_map_lights(commands, root, sco);
        let scripts = template.filter(|t| t.has_scripts).and_then(|t| {
            let engine = t.engine.lock().ok()?.fork();
            Some(SceneryScripts::new(
                engine,
                &t.string_names,
                &obj.string_vars,
                t.diagnostics.clone(),
            ))
        });
        let mut input = SceneryObjectInput {
            sco,
            mesh_parts,
            scripts,
            signal: None,
        };
        if !needs_runtime(&input) {
            let parts: Vec<ModelPart> = input.mesh_parts.into_iter().flatten().collect();
            let spawned = attach_model_parts(commands, root, &parts, "Mesh");
            opts_bevy::bind_alpha_scale(commands, &spawned, &parts);
            return;
        }
        if input.scripts.is_some() {
            self.scenery.stats.scripted_objects += 1;
        }
        input.signal = self.signal_for(obj, sco);
        let mut assets = SceneryBuildAssets {
            meshes,
            materials,
            lights: &mut self.scenery.lights,
        };
        build_scenery_object(commands, root, input, &mut assets);
    }
}
