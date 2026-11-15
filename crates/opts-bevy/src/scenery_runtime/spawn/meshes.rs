//! The mesh entities of an object: the mesh rules, the animations and the parts under each mesh.

use super::*;

pub(super) struct MeshBuildArgs<'a, 'w, 's> {
    pub(super) commands: &'a mut Commands<'w, 's>,
    pub(super) root: Entity,
    pub(super) parent: Entity,
    pub(super) sco: &'a SceneryObject,
    pub(super) index: usize,
    pub(super) parts: &'a [ModelPart],
    pub(super) engine: Option<&'a mut ScriptEngine>,
    pub(super) rt: &'a mut SceneryObjectRuntime,
}

/// Builds the mesh entities of `[mesh]` number `i` under `parent`; returns the rule entity.
pub(super) fn build_mesh(mut args: MeshBuildArgs<'_, '_, '_>) {
    let def = &args.sco.meshes[args.index];
    let pivot = args.parts.iter().find_map(|p| p.pivot).map(to_omsi);
    let steps: Vec<_> = def
        .animations
        .iter()
        .flat_map(|a| anim_steps(a, if a.origin_from_mesh { pivot } else { None }))
        .collect();
    let visible: Vec<(String, f32)> = def.visible.iter().cloned().collect();
    let holder = if steps.is_empty() && visible.is_empty() {
        args.parent
    } else {
        let vars = steps
            .iter()
            .map(|s| s.variable.as_str())
            .chain(visible.iter().map(|v| v.0.as_str()))
            .map(|n| resolve(n, args.engine.as_deref_mut()))
            .collect();
        let rule = SceneryMeshRule {
            anim: MeshAnimation {
                owner: args.root,
                steps,
                visible,
                shown: Vec::new(),
            },
            vars,
        };
        let e = args
            .commands
            .spawn((
                Transform::default(),
                Visibility::Inherited,
                rule,
                Name::new(format!("Mesh_{}", args.index)),
            ))
            .id();
        args.commands.entity(args.parent).add_child(e);
        args.rt.meshes.push(e);
        e
    };
    let spawned = attach_model_parts(args.commands, holder, args.parts, &def.file);
    super::super::bind_alpha_scale(args.commands, &spawned, args.parts);
    // `[mouseevent]`: a click on the mesh raises the trigger (the tongue of rail points).
    let click = def
        .mouse_event
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty());
    for (e, part) in spawned.into_iter().zip(args.parts) {
        if let Some(trigger) = click {
            args.commands
                .entity(e)
                .insert(crate::vehicle_triggers::SceneryMouseTrigger {
                    owner: args.root,
                    trigger: trigger.to_owned(),
                });
        }
        if let Some(var) = &part.light_variable {
            let in_place = var.eq_ignore_ascii_case(NIGHT_VARIABLE);
            args.commands.entity(e).insert(SceneryLightMap {
                var: resolve(var, args.engine.as_deref_mut()),
                in_place,
                applied: f32::NAN,
            });
            args.rt.light_maps.push(e);
        }
    }
}
