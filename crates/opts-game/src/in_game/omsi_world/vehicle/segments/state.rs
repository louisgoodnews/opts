//! Per-segment state and the per-mesh stages of `load_segment_meshes_for`.

use super::super::skins::{
    anim_pivots, articulation_runs, bind_skins, forms_cycle, is_bone_stem, mesh_pivot, pick_run,
    pivot_of, SkinBinding,
};
use super::overrides::{build_overrides, in_file_order, mesh_stem};
use super::*;

type ModelMeshDef = opts_core::formats::vehicles::model::ModelMesh;

/// The asset stores the meshes of a segment are built into.
pub(in super::super) struct SegmentAssets<'a> {
    pub(in super::super) meshes: &'a mut Assets<Mesh>,
    pub(in super::super) materials: &'a mut Assets<StandardMaterial>,
    pub(in super::super) images: &'a mut Assets<Image>,
}

/// What one mesh of a segment turned into once its o3d parts are loaded.
struct LoadedMesh {
    parts: Vec<opts_bevy::ModelPart>,
    skinned: bool,
}

/// The inputs of one segment load and the state its meshes build up.
pub(in super::super) struct SegmentState<'a> {
    root: Entity,
    owner: Entity,
    lod0: &'a opts_core::formats::vehicles::model::ModelLod,
    model_dir: &'a Path,
    tex_dirs: &'a [PathBuf],
    // Names of the bones the skinned meshes of this model list (`[setbone]`): a mesh with that
    // file name is the dummy that carries the bone's animation (`Gelenk_A.o3d` for `Gelenk_A`).
    bone_names: std::collections::HashSet<String>,
    sibling_run: Vec<bool>,
    // `[mesh_ident]` / `[animparent]`: a mesh inherits the animation of its parent.
    pivots: std::collections::HashMap<String, Entity>,
    mesh_count: usize,
    vertex_count: usize,
    min: Vec3,
    max: Vec3,
    // Dynamic text textures of this segment: one image per `[texttexture]` index, redrawn when the
    // script string variable it shows changes (`vehicle_text.rs`).
    text_images: std::collections::HashMap<usize, Handle<Image>>,
    script_images: std::collections::HashMap<usize, Handle<Image>>,
    // Dummy meshes (`…Dum_Gelenk_A`) that carry a bone's animation, and the skinned meshes
    // (`[smoothskin]` + `[setbone]`: the articulation bellows) bound to them.
    dummies: Vec<(String, Entity)>,
    skins: Vec<SkinBinding>,
    // Runs of `bone_<letter>` dummies: a skinned mesh numbers its bones (`[setbone] name n`)
    // along the run of dummies declared before it (the NF6D-style bellows of the Heuliez GX).
    // (shared stem without the last letter, dummies in declaration order); the run touched last
    // belongs to the skinned mesh that follows.
    bone_runs: Vec<(String, Vec<Entity>)>,
    last_run: Option<usize>,
    // How often a mesh of each file name came before (see `in_file_order`).
    repeats: std::collections::HashMap<String, u32>,
}

/// Shadow-only and AI-only meshes are not part of the visible player model; the developer aid
/// `OPTS_HIDE_MESH=<substring>[,<substring>]` also skips matching meshes (to bisect rendering
/// problems).
fn is_skipped(mesh_def: &ModelMeshDef) -> bool {
    let vp = mesh_def.viewpoint;
    if mesh_def.isshadow || !(vp.is_outside() || vp.is_inside()) {
        return true;
    }
    std::env::var("OPTS_HIDE_MESH").is_ok_and(|v| {
        v.split(',')
            .filter(|t| !t.is_empty())
            .any(|t| mesh_def.path.to_lowercase().contains(&t.to_lowercase()))
    })
}

impl<'a> SegmentState<'a> {
    pub(in super::super) fn new(
        commands: &mut Commands,
        (root, owner): (Entity, Entity),
        lod0: &'a opts_core::formats::vehicles::model::ModelLod,
        (model_dir, tex_dirs): (&'a Path, &'a [PathBuf]),
    ) -> Self {
        let bone_names = lod0
            .meshes
            .iter()
            .flat_map(|m| m.bones.iter().map(|b| b.name.to_lowercase()))
            .collect();
        Self {
            root,
            owner,
            lod0,
            model_dir,
            tex_dirs,
            bone_names,
            sibling_run: articulation_runs(&lod0.meshes),
            pivots: anim_pivots(commands, root, &lod0.meshes),
            mesh_count: 0,
            vertex_count: 0,
            min: Vec3::splat(f32::MAX),
            max: Vec3::splat(f32::MIN),
            text_images: Default::default(),
            script_images: Default::default(),
            dummies: Vec::new(),
            skins: Vec::new(),
            bone_runs: Vec::new(),
            last_run: None,
            repeats: Default::default(),
        }
    }

    /// Loads every visible mesh of the segment, then binds the skinned ones to their bones.
    pub(in super::super) fn load_all(
        mut self,
        job: &mut WorldLoadJob,
        commands: &mut Commands,
        assets: &mut SegmentAssets,
    ) -> (usize, usize, Vec3, Vec3) {
        for (mesh_index, mesh_def) in self.lod0.meshes.iter().enumerate() {
            if is_skipped(mesh_def) {
                continue;
            }
            self.load_mesh(job, commands, assets, mesh_index);
        }
        bind_skins(commands, self.root, &self.dummies, self.skins);
        (self.mesh_count, self.vertex_count, self.min, self.max)
    }

    fn animation_of(
        &self,
        job: &WorldLoadJob,
        mesh_def: &ModelMeshDef,
    ) -> Option<opts_bevy::MeshAnimation> {
        opts_bevy::MeshAnimation::from_mesh(
            self.owner,
            mesh_def,
            mesh_pivot(job, &mesh_def.path, self.model_dir, mesh_def),
        )
    }

    fn load_mesh(
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
        let Some(loaded) = self.load_parts(job, commands, assets, mesh_def) else {
            return;
        };
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
    fn frame_of(&self, mesh_def: &ModelMeshDef) -> Entity {
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
    fn decorate(
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
            // script variable that is 0 stays out; dirt and rain overlays are not built).
            entity.insert((
                opts_bevy::AlphaScaleBinding::new(owner, var.clone()),
                Visibility::Hidden,
            ));
        }
        if let Some(trigger) = hot_spot {
            entity.insert(opts_bevy::MouseTrigger {
                owner,
                trigger: trigger.to_owned(),
            });
        }
    }

    /// A dummy mesh only carries the animation of a bone. Its geometry is often protected
    /// (scrambled) or empty, so the joint is an entity of its own.
    fn register_joint(&mut self, job: &WorldLoadJob, commands: &mut Commands, mesh_index: usize) {
        let mesh_def = &self.lod0.meshes[mesh_index];
        let stem = Path::new(&mesh_def.path.replace('\\', "/"))
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        // Numbered siblings driven by `articulation_*` (`Dum_A`…`Dum_D`, `Gelenk_Dummy_A`…,
        // `przd_a`…) are the bones' dummies even when their names say nothing about bones.
        let is_bone_dummy = is_bone_stem(&stem) || self.sibling_run[mesh_index];
        let carries_animation = mesh_def.anims.iter().any(|a| a.driver_variable().is_some());
        if !((stem.contains("dum") || is_bone_dummy || self.bone_names.contains(&stem))
            && carries_animation)
        {
            return;
        }
        let mut joint = commands.spawn((
            Name::new(format!("joint_{stem}")),
            Transform::IDENTITY,
            Visibility::Inherited,
        ));
        if let Some(anim) = self.animation_of(job, mesh_def) {
            joint.insert(anim);
        }
        let id = joint.id();
        commands.entity(self.root).add_child(id);
        self.dummies.push((stem.clone(), id));
        if is_bone_dummy {
            self.push_bone_run(&stem, id);
        }
    }

    /// Appends the dummy `id` to the run of its stem without the last letter.
    fn push_bone_run(&mut self, stem: &str, id: Entity) {
        let mut chars = stem.chars();
        chars.next_back();
        let prefix = chars.as_str().to_owned();
        let at = match self.bone_runs.iter().position(|(k, _)| *k == prefix) {
            Some(at) => at,
            None => {
                self.bone_runs.push((prefix, Vec::new()));
                self.bone_runs.len() - 1
            }
        };
        self.bone_runs[at].1.push(id);
        self.last_run = Some(at);
    }

    /// Loads the o3d parts of one mesh; `None` when the file is missing.
    fn load_parts(
        &mut self,
        job: &mut WorldLoadJob,
        commands: &mut Commands,
        assets: &mut SegmentAssets,
        mesh_def: &ModelMeshDef,
    ) -> Option<LoadedMesh> {
        let Some(o3d_path) = job.install.find_in(self.model_dir, &mesh_def.path) else {
            tracing::error!(at = %opts_bevy::position_tag(), mesh = %mesh_def.path, "vehicle o3d not found");
            job.cache
                .stats
                .failures
                .push(format!("vehicle o3d not found: {}", mesh_def.path));
            return None;
        };
        let overrides = build_overrides(
            (job, commands, assets.images),
            (mesh_def, self.owner, self.root),
            self.tex_dirs,
            (&mut self.text_images, &mut self.script_images),
        );
        let mut loader = ContentLoader {
            install: &job.install,
            cache: &mut job.cache,
            meshes: assets.meshes,
            materials: assets.materials,
            images: assets.images,
        };
        super::super::super::trace::mesh_used(&o3d_path);
        let skinned = mesh_def.smoothskin && !mesh_def.bones.is_empty();
        let parts = if skinned {
            let slots: Vec<(String, u16)> = mesh_def
                .bones
                .iter()
                .filter_map(|b| u16::try_from(b.index).ok().map(|i| (b.name.clone(), i)))
                .collect();
            loader.o3d_skinned_parts_slots(&o3d_path, self.tex_dirs, &overrides, &slots)
        } else {
            loader.o3d_parts(&o3d_path, self.tex_dirs, &overrides)
        };
        let copy = *self
            .repeats
            .entry(mesh_stem(&mesh_def.path))
            .and_modify(|n| *n += 1)
            .or_insert(0);
        let parts = in_file_order(job, assets.materials, parts, copy);
        Some(LoadedMesh { parts, skinned })
    }

    /// Counts the parts and grows the bounding box by the ones that belong to the body.
    fn grow_bounds(&mut self, meshes: &Assets<Mesh>, parts: &[opts_bevy::ModelPart]) {
        for part in parts {
            if let Some(mesh) = meshes.get(&part.mesh) {
                if let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                {
                    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                    for p in pos {
                        lo = lo.min(Vec3::from(*p));
                        hi = hi.max(Vec3::from(*p));
                    }
                    // Camera framing uses the vehicle body only: meshes that are huge or far
                    // from the origin (shadow volumes, sky/helper geometry) would skew it.
                    if (hi - lo).max_element() < 16.0 && ((hi + lo) * 0.5).abs().max_element() < 8.0
                    {
                        self.min = self.min.min(lo);
                        self.max = self.max.max(hi);
                    }
                }
            }
            self.vertex_count += part.vertices;
            self.mesh_count += 1;
        }
    }
}
