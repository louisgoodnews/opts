//! Builds the runtime entity hierarchy of a placed scenery object from its `.sco` and its loaded
//! mesh parts (`docs/src/architecture/scenery-runtime.md`, "Entity layout").

use bevy::prelude::*;
use opts_core::formats::scenery::{SceneryObject, ScoLight};
use opts_sim::script::ScriptEngine;
use opts_traffic::SignalProgram;

use super::clock::{HostVar, VarRef};
use super::effects::{SceneryLight, SceneryLightMap, SceneryMeshRule, SceneryObjectRuntime};
use super::lights::{spawn_light_sprite, SceneryLightAssets};
use super::lod::SceneryLod;
use super::scripts::SceneryScripts;
use crate::coords::omsi_to_bevy_pos;
use crate::model::{attach_model_parts, ModelPart};
use crate::vehicle_anim::{anim_steps_with, MeshAnimation};

/// Variable that switches light maps which name none (night lighting).
pub const NIGHT_VARIABLE: &str = "NightlightA";

/// What a placed object brings to [`build_scenery_object`].
pub struct SceneryObjectInput<'a> {
    /// The parsed `.sco`.
    pub sco: &'a SceneryObject,
    /// Loaded parts of each `[mesh]`, in `.sco` order (empty for a mesh that failed).
    pub mesh_parts: Vec<Vec<ModelPart>>,
    /// The object's script engine, if it has scripts.
    pub scripts: Option<SceneryScripts>,
    /// The light program behind `TrafficLightPhase`.
    pub signal: Option<SignalProgram>,
}

/// Asset stores and shared light assets used while building.
pub struct SceneryBuildAssets<'a> {
    /// Mesh store (bounds for LOD, the light sphere).
    pub meshes: &'a mut Assets<Mesh>,
    /// Material store (light colours).
    pub materials: &'a mut Assets<StandardMaterial>,
    /// Shared light sprite assets.
    pub lights: &'a mut SceneryLightAssets,
}

/// Whether an object needs the runtime hierarchy (otherwise its parts hang flat under the root,
/// as before, and it costs nothing at run time).
#[must_use]
pub fn needs_runtime(input: &SceneryObjectInput<'_>) -> bool {
    let sco = input.sco;
    input.scripts.is_some()
        || !sco.lights.is_empty()
        || sco
            .meshes
            .iter()
            .any(|m| m.lod_threshold.is_some() || m.visible.is_some() || !m.animations.is_empty())
        || input
            .mesh_parts
            .iter()
            .flatten()
            .any(|p| p.light_variable.is_some())
}

pub(super) fn resolve(name: &str, engine: Option<&mut ScriptEngine>) -> VarRef {
    match engine {
        Some(e) => VarRef::Engine(e.variable_id(name)),
        None => VarRef::Host(HostVar::of(name)),
    }
}

/// Bevy position of an object-local OMSI point.
pub(super) fn local(p: Vec3) -> Vec3 {
    omsi_to_bevy_pos(p)
}


mod groups;
mod meshes;
mod sprites;
use groups::*;
use meshes::*;
use sprites::*;

/// Builds the runtime hierarchy of an object under `root` (which carries the placement
/// transform) and inserts its runtime components. Returns the number of part entities.
pub fn build_scenery_object(
    commands: &mut Commands,
    root: Entity,
    mut input: SceneryObjectInput<'_>,
    assets: &mut SceneryBuildAssets<'_>,
) -> usize {
    let sco = input.sco;
    let groups = lod_groups(commands, root, sco);
    let parent_of = |t: Option<f32>| {
        t.and_then(|t| groups.iter().find(|g| g.1 == t).map(|g| g.0))
            .unwrap_or(root)
    };
    let mut rt = SceneryObjectRuntime {
        signal: input.signal.take(),
        ..Default::default()
    };
    let mut engine = input.scripts.as_mut().and_then(SceneryScripts::engine_mut);
    let mut count = 0;
    for (i, parts) in input.mesh_parts.iter().enumerate() {
        if parts.is_empty() || i >= sco.meshes.len() {
            continue;
        }
        let parent = parent_of(sco.meshes[i].lod_threshold);
        build_mesh(MeshBuildArgs {
            commands,
            root,
            parent,
            sco,
            index: i,
            parts,
            engine: engine.as_deref_mut(),
            rt: &mut rt,
        });
        count += parts.len();
    }
    for light in &sco.lights {
        // Under the object root, whatever `[LOD]` band the light was declared in.
        let e = build_light(commands, root, light, engine.as_deref_mut(), assets);
        rt.lights.push(e);
    }
    let mut root_cmd = commands.entity(root);
    if sco.no_map_lighting {
        root_cmd.insert(crate::light_rooms::NoMapLighting);
    }
    if !groups.is_empty() {
        let (centre, radius) = bounds(&input.mesh_parts, assets.meshes);
        root_cmd.insert(SceneryLod::new(groups, centre, radius));
    }
    root_cmd.insert(rt);
    if let Some(scripts) = input.scripts {
        root_cmd.insert(scripts);
    }
    count
}
