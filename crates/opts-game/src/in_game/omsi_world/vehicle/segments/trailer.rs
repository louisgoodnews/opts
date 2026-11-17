//! The trailer end of an articulated head's bellow: its trailer-end vertices move to the
//! trailer-front joint, which follows the coupled trailer (`opts_bevy::articulation`).

use super::super::skins::joint_slots;
use super::state::ModelMeshDef;
use super::*;

impl<'a> SegmentState<'a> {
    /// The hitch of the trailer-front joint when this skinned mesh is the bellow of the head's
    /// coupled trailer: the trailer-end vertices of its parts are moved from ring D to the joint
    /// after the skin's last joint. `None` for every other skinned mesh, and for a head without
    /// a trailer.
    pub(super) fn trailer_front(
        &self,
        meshes: &mut Assets<Mesh>,
        mesh_index: usize,
        parts: &[opts_bevy::ModelPart],
    ) -> Option<Vec3> {
        let link = self.trailer?;
        let mesh_def: &ModelMeshDef = &self.lod0.meshes[mesh_index];
        // A bellow is a skinned mesh with its bones (`[setbone]`): the articulation drives its dummies,
        // not the bellow itself, so the bellow carries no driver of its own.
        if mesh_def.bones.len() < 2 {
            return None;
        }
        // The trailer end faces away from the hitch: the trailer-end axis (OMSI) in the model space.
        let axis = opts_bevy::mesh::trailer_end_axis(link.coupling, link.front)?;
        let bones = u16::try_from(joint_slots(&mesh_def.bones)).ok()?;
        let trailer_joint = bones.checked_add(1)?;
        let handles: Vec<Handle<Mesh>> = parts.iter().map(|p| p.mesh.clone()).collect();
        let ring = opts_bevy::mesh::remap_bellow_trailer_end(
            meshes,
            &handles,
            opts_bevy::mesh::model_dir_from_omsi(axis),
            (bones, trailer_joint),
        )?;
        tracing::info!(
            ring,
            trailer_joint,
            parts = handles.len(),
            "bellow trailer end on the trailer-front joint"
        );
        Some(link.coupling)
    }
}
