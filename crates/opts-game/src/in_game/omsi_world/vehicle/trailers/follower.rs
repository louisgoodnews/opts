//! One following unit of a trailer chain: its model, articulated segment, scripts and meshes.

use super::super::super::consist_plan::PlannedPart;
use super::*;
use opts_bevy::vehicle::{exhaust_outlet, ExhaustOutlet};

mod model;
use model::{load_model, rear_axle_y, set_job_textures};
mod segment;
use segment::{account_load, articulated_segment};
mod extras;
use extras::{attach_meshes, attach_unit_extras};

/// The three asset stores the unit's meshes are built into.
pub(super) type MeshAssets<'a> = (
    &'a mut Assets<Mesh>,
    &'a mut Assets<StandardMaterial>,
    &'a mut Assets<Image>,
);

/// What the chain collects while its units are spawned one after the other.
pub(super) struct ChainState {
    pub cams: CabCameras,
    pub driver_cams: CabCameras,
    /// Mass of the unit in front: what a trailer without a stated mass is assumed to weigh.
    pub previous_kg: f32,
    pub chain: Vec<Entity>,
    pub layout: Vec<super::super::super::rail::Part>,
    /// Script state: the head's engine, then the units with scripts of their own. A unit without
    /// `[scriptshare]` and with scripts runs an engine on its own entity; the others read the
    /// engine of the unit before them that owns a store (`owner`).
    pub owner: Entity,
    pub consist: Vec<opts_bevy::ConsistUnit>,
    /// Axles counted so far: trailer models number theirs after the head's (`Wheel_Rotation_2_L`).
    pub axle_base: usize,
    /// Rail: the head's mass (at least 20 t), then the stated mass of every following unit and
    /// whether it pulls; a unit that states none adds nothing.
    pub powered_kg: f32,
    pub total_kg: f32,
}

/// Spawns one following unit behind the chain so far. `false` when it cannot be loaded and the
/// chain ends here.
pub(super) fn spawn_follower(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    assets: MeshAssets,
    root: Entity,
    (part, rail): (&PlannedPart, bool),
    st: &mut ChainState,
) -> bool {
    let (path, manifest, dir) = (&part.path, &part.manifest, &part.dir);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (cfg, model) = match load_model(job, manifest, dir) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(at = %opts_bevy::position_tag(), trailer = %path.display(), "cannot load trailer: {e}");
            return false;
        }
    };
    let Some(lod0) = model.detail_lod() else {
        return false;
    };
    set_job_textures(job, &model);
    let load = account_load(st, part, rail);
    let seg = commands
        .spawn((
            Name::new(name.clone()),
            Transform::default(),
            Visibility::default(),
            articulated_segment(part, manifest, &load, st.axle_base),
            InGameEntity,
        ))
        .id();
    st.chain.push(seg);
    if let Some(outlet) = exhaust_outlet(&model) {
        commands.entity(seg).insert(ExhaustOutlet(outlet));
    }
    attach_unit_extras(job, commands, (root, seg), st, part);
    let (count, verts) = attach_meshes(
        job,
        commands,
        assets,
        (seg, st.owner),
        &lod0,
        (&cfg, &model, dir),
    );
    tracing::info!(trailer = %name, meshes = count, vertices = verts, "trailer segment attached");
    st.layout.push(super::super::super::rail::Part {
        entity: seg,
        name,
        behind: part.behind,
        half_span: manifest.bogie_half_span.unwrap_or(0.0),
        reversed: part.turned,
        meshes: count,
        vertices: verts,
        body: manifest
            .bounding_box
            .map(|(size, centre)| super::super::super::BodyBox::from_bounding_box(size, centre)),
    });
    st.cams
        .extend(manifest.pax_cameras.iter().cloned().map(|c| (seg, c)));
    // A unit with a cab of its own: its driver cameras, for the cab switch.
    st.driver_cams
        .extend(manifest.driver_cameras.iter().cloned().map(|c| (seg, c)));
    true
}

#[cfg(test)]
mod tests;
