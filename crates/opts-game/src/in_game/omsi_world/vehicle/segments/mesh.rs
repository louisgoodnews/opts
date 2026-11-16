//! One mesh of a segment: its frame and pivot, its parts and the script components they carry.

use super::super::skins::{forms_cycle, pick_run, pivot_of, SkinBinding};
use super::state::{ModelMeshDef, SegmentState};
use super::*;

impl<'a> SegmentState<'a> {
    pub(super) fn load_mesh(
        &mut self,
        job: &mut WorldLoadJob,
        commands: &mut Commands,
        assets: &mut SegmentAssets,
        mesh_index: usize,
    ) {
        let mesh_def = &self.lod0.meshes[mesh_index];
        // The frame this mesh lives in: its parent's pivot (`[animparent]`), else the vehicle. A mesh
        // that others hang on (`[mesh_ident]`) animates its pivot, and its parts sit below it.
        let own_pivot = pivot_of(&self.pivots, mesh_def.mesh_ident.as_deref());
        let frame = self.frame_of(mesh_def);
        if let Some(pivot) = own_pivot {
            commands.entity(frame).add_child(pivot);
            if let Some(anim) = self.animation_of(job, mesh_def) {
                commands.entity(pivot).insert(anim);
            }
        }
        let parts_parent = own_pivot.unwrap_or(frame);
        self.register_joint(job, commands, mesh_index);
        let Some(mut loaded) = self.load_parts(job, commands, assets, mesh_def) else {
            return;
        };
        // Weather overlays the "Window and body overlays" setting drops are never built.
        loaded.parts.retain(opts_bevy::overlays::part_is_built);
        self.grow_bounds(assets.meshes, &loaded.parts);
        let spawned = attach_model_parts(commands, parts_parent, &loaded.parts, &mesh_def.path);
        // Script variables move and show the mesh (`[newanim]`, `[visible]`) and switch its light
        // maps; a click on it raises its `[mouseevent]` trigger. A pivot already carries the
        // animation of its parts.
        let anim = if own_pivot.is_some() {
            None
        } else {
            self.animation_of(job, mesh_def)
        };
        let view_scope = opts_bevy::ViewScope::of(mesh_def.viewpoint);
        let hot_spot = super::super::hot_spots::hot_spot_of(&self.lod0.meshes, mesh_index);
        if loaded.skinned {
            self.skins.push(SkinBinding {
                entities: spawned.clone(),
                bones: mesh_def.bones.clone(),
                run: pick_run(&self.bone_runs, self.last_run, &mesh_def.bones),
                frame: parts_parent,
            });
        }
        for (e, part) in spawned.iter().copied().zip(&loaded.parts) {
            let mut entity = commands.entity(e);
            self.decorate(&mut entity, part, (&anim, view_scope, hot_spot));
        }
    }
    /// The pivot of the mesh's parent (`[animparent]`), else the vehicle root.
    pub(super) fn frame_of(&self, mesh_def: &ModelMeshDef) -> Entity {
        pivot_of(&self.pivots, mesh_def.animparent.as_deref())
            .filter(|_| {
                !forms_cycle(
                    &self.lod0.meshes,
                    mesh_def.mesh_ident.as_deref(),
                    mesh_def.animparent.as_deref(),
                )
            })
            .unwrap_or(self.root)
    }
    /// Inserts the script-driven components of one spawned part.
    pub(super) fn decorate(
        &self,
        entity: &mut EntityCommands,
        part: &opts_bevy::ModelPart,
        (anim, view_scope, hot_spot): (
            &Option<opts_bevy::MeshAnimation>,
            Option<opts_bevy::ViewScope>,
            Option<&str>,
        ),
    ) {
        let owner = self.owner;
        if let Some(anim) = anim {
            entity.insert(anim.clone());
        }
        if let Some(scope) = view_scope {
            entity.insert(scope);
        }
        if let Some(var) = &part.light_variable {
            entity.insert(opts_bevy::LightMapBinding::new(owner, var.clone()));
        }
        if let Some((var, light_map)) = &part.free_texture {
            entity.insert(opts_bevy::free_texture::FreeTextureBinding::new(
                owner,
                var.clone(),
                self.tex_dirs.to_vec(),
                light_map.clone(),
            ));
        }
        if let Some(var) = &part.alpha_variable {
            // Starts hidden: the variable decides on the first frame (glass scaled by a
            // script variable that is 0 stays out; dirt and rain overlays are only built where the overlay setting keeps them).
            entity.insert((
                opts_bevy::AlphaScaleBinding::new(owner, var.clone()),
                Visibility::Hidden,
            ));
        }
        if let Some(kind) = opts_bevy::overlays::part_kind(part) {
            entity.insert(kind);
        }
        if let Some(trigger) = hot_spot {
            entity.insert(opts_bevy::MouseTrigger {
                owner,
                trigger: trigger.to_owned(),
            });
        }
    }
}
