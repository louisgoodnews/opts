//! Per-segment state and the per-mesh stages of `load_segment_meshes_for`.

use super::super::skins::{
    anim_pivots, articulation_runs, bind_skins, mesh_pivot, SkinBinding, TrailerLink,
};
use super::*;

pub(super) type ModelMeshDef = opts_core::formats::vehicles::model::ModelMesh;

/// The asset stores the meshes of a segment are built into.
pub(in super::super) struct SegmentAssets<'a> {
    pub(in super::super) meshes: &'a mut Assets<Mesh>,
    pub(in super::super) materials: &'a mut Assets<StandardMaterial>,
    pub(in super::super) images: &'a mut Assets<Image>,
}

/// What one mesh of a segment turned into once its o3d parts are loaded.
pub(super) struct LoadedMesh {
    pub(super) parts: Vec<opts_bevy::ModelPart>,
    pub(super) skinned: bool,
}

/// The inputs of one segment load and the state its meshes build up.
pub(in super::super) struct SegmentState<'a> {
    pub(super) root: Entity,
    pub(super) owner: Entity,
    pub(super) lod0: &'a opts_core::formats::vehicles::model::ModelLod,
    pub(super) model_dir: &'a Path,
    pub(super) tex_dirs: &'a [PathBuf],
    // Names of the bones the skinned meshes of this model list (`[setbone]`): a mesh with that
    // file name is the dummy that carries the bone's animation (`Gelenk_A.o3d` for `Gelenk_A`).
    pub(super) bone_names: std::collections::HashSet<String>,
    pub(super) sibling_run: Vec<bool>,
    // `[mesh_ident]` / `[animparent]`: a mesh inherits the animation of its parent.
    pub(super) pivots: std::collections::HashMap<String, Entity>,
    pub(super) mesh_count: usize,
    pub(super) vertex_count: usize,
    pub(super) min: Vec3,
    pub(super) max: Vec3,
    // Dynamic text textures of this segment: one image per `[texttexture]` index, redrawn when the
    // script string variable it shows changes (`vehicle_text.rs`).
    pub(super) text_images: super::overrides::TextImageMap,
    pub(super) script_images: super::overrides::ImageMap,
    // Dummy meshes (`…Dum_Gelenk_A`) that carry a bone's animation, and the skinned meshes
    // (`[smoothskin]` + `[setbone]`: the articulation bellows) bound to them.
    pub(super) dummies: Vec<(String, Entity)>,
    pub(super) skins: Vec<SkinBinding>,
    // Runs of `bone_<letter>` dummies: a skinned mesh numbers its bones (`[setbone] name n`)
    // along the run of dummies declared before it (the NF6D-style bellows of the Heuliez GX).
    // (shared stem without the last letter, dummies in declaration order); the run touched last
    // belongs to the skinned mesh that follows.
    pub(super) bone_runs: Vec<(String, Vec<Entity>)>,
    pub(super) last_run: Option<usize>,
    // How often a mesh of each file name came before (see `in_file_order`).
    pub(super) repeats: std::collections::HashMap<String, u32>,
    // The coupled trailer of an articulated head: its bellow's trailer end follows it.
    pub(super) trailer: Option<TrailerLink>,
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
            trailer: None,
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

    pub(super) fn animation_of(
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
}
