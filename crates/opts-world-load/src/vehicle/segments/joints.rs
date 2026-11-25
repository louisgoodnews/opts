//! The bone dummies of a segment: the joint entities that carry a bone's animation, and the runs of

use super::super::skins::is_bone_stem;
use super::state::SegmentState;
use super::*;

impl<'a> SegmentState<'a> {
    pub(super) fn register_joint(
        &mut self,
        job: &WorldLoadJob,
        commands: &mut Commands,
        mesh_index: usize,
    ) {
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
    pub(super) fn push_bone_run(&mut self, stem: &str, id: Entity) {
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
}
