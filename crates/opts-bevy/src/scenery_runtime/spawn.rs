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
use crate::vehicle_anim::{anim_steps, MeshAnimation};

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

fn resolve(name: &str, engine: Option<&mut ScriptEngine>) -> VarRef {
    match engine {
        Some(e) => VarRef::Engine(e.variable_id(name)),
        None => VarRef::Host(HostVar::of(name)),
    }
}

/// Bevy position of an object-local OMSI point.
fn local(p: Vec3) -> Vec3 {
    omsi_to_bevy_pos(p)
}

/// Bevy → OMSI axes for a pivot.
fn to_omsi(p: Vec3) -> Vec3 {
    Vec3::new(p.x, -p.z, p.y)
}

/// Spawns the LOD group entities (one per distinct mesh threshold) under `root`. Light sprites
/// are not part of a LOD group: a lantern's `[light_enh_2]` written after `[LOD] 0` would
/// otherwise vanish with its mesh band within ~70 m, while the glow is what is seen from afar.
fn lod_groups(commands: &mut Commands, root: Entity, sco: &SceneryObject) -> Vec<(Entity, f32)> {
    let mut thresholds: Vec<f32> = Vec::new();
    let all = sco.meshes.iter().map(|m| m.lod_threshold);
    for t in all.flatten() {
        if !thresholds.contains(&t) {
            thresholds.push(t);
        }
    }
    thresholds
        .into_iter()
        .map(|t| {
            let e = commands
                .spawn((
                    Transform::default(),
                    Visibility::Inherited,
                    Name::new(format!("LOD_{t}")),
                ))
                .id();
            commands.entity(root).add_child(e);
            (e, t)
        })
        .collect()
}

/// Bounding sphere (centre, radius) of all parts in the object's frame.
fn bounds(parts: &[Vec<ModelPart>], meshes: &Assets<Mesh>) -> (Vec3, f32) {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for part in parts.iter().flatten() {
        let positions = meshes
            .get(&part.mesh)
            .and_then(|m| m.attribute(Mesh::ATTRIBUTE_POSITION));
        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) = positions {
            for p in pos {
                lo = lo.min(Vec3::from(*p));
                hi = hi.max(Vec3::from(*p));
            }
        }
    }
    if lo.x > hi.x {
        return (Vec3::ZERO, 1.0);
    }
    ((lo + hi) * 0.5, ((hi - lo) * 0.5).length().max(0.1))
}

/// Builds the mesh entities of `[mesh]` number `i` under `parent`; returns the rule entity.
#[allow(clippy::too_many_arguments)]
fn build_mesh(
    commands: &mut Commands,
    root: Entity,
    parent: Entity,
    sco: &SceneryObject,
    i: usize,
    parts: &[ModelPart],
    mut engine: Option<&mut ScriptEngine>,
    rt: &mut SceneryObjectRuntime,
) {
    let def = &sco.meshes[i];
    let pivot = parts.iter().find_map(|p| p.pivot).map(to_omsi);
    let steps: Vec<_> = def
        .animations
        .iter()
        .flat_map(|a| anim_steps(a, if a.origin_from_mesh { pivot } else { None }))
        .collect();
    let visible: Vec<(String, f32)> = def.visible.iter().cloned().collect();
    let holder = if steps.is_empty() && visible.is_empty() {
        parent
    } else {
        let vars = steps
            .iter()
            .map(|s| s.variable.as_str())
            .chain(visible.iter().map(|v| v.0.as_str()))
            .map(|n| resolve(n, engine.as_deref_mut()))
            .collect();
        let rule = SceneryMeshRule {
            anim: MeshAnimation {
                owner: root,
                steps,
                visible,
                shown: Vec::new(),
            },
            vars,
        };
        let e = commands
            .spawn((
                Transform::default(),
                Visibility::Inherited,
                rule,
                Name::new(format!("Mesh_{i}")),
            ))
            .id();
        commands.entity(parent).add_child(e);
        rt.meshes.push(e);
        e
    };
    let spawned = attach_model_parts(commands, holder, parts, &def.file);
    super::bind_alpha_scale(commands, &spawned, parts);
    for (e, part) in spawned.into_iter().zip(parts) {
        if let Some(var) = &part.light_variable {
            let in_place = var.eq_ignore_ascii_case(NIGHT_VARIABLE);
            commands.entity(e).insert(SceneryLightMap {
                var: resolve(var, engine.as_deref_mut()),
                in_place,
                applied: f32::NAN,
            });
            rt.light_maps.push(e);
        }
    }
}

/// Spawns the sprite of one light effect under `parent`.
fn build_light(
    commands: &mut Commands,
    parent: Entity,
    light: &ScoLight,
    engine: Option<&mut ScriptEngine>,
    assets: &mut SceneryBuildAssets<'_>,
) -> Entity {
    let var = if light.variable.trim().is_empty() {
        VarRef::One
    } else {
        resolve(light.variable.trim(), engine)
    };
    // Cone angles are full angles (as for Direct3D spotlights) ❓.
    let half = |deg: f32| (deg.clamp(0.0, 360.0) * 0.5).to_radians().cos();
    let sprite = SceneryLight {
        var,
        factor: light.factor,
        size_m: light.size_m.max(0.01),
        direction: if light.omni {
            Vec3::ZERO
        } else {
            local(light.direction).normalize_or_zero()
        },
        cos_inner: half(light.cone_inner_deg),
        cos_outer: half(light.cone_outer_deg),
        level: 0.0,
    };
    let mesh = assets.lights.glow_mesh(assets.meshes);
    let material = assets.lights.material(assets.materials, light.color);
    let e = spawn_light_sprite(commands, sprite, local(light.position), mesh, material);
    commands.entity(parent).add_child(e);
    // A directional light with the `cone` flag also draws a visible cone, next to the glow; its
    // brightness is the glow's level (the same variable, switched by the same night rule).
    if let Some(shape) = crate::light_cones::lantern_cone_shape(light) {
        crate::light_cones::spawn_light_cone(
            commands,
            parent,
            crate::light_cones::LightCone {
                kind: crate::light_cones::ConeKind::Lantern,
                shape,
                level: crate::light_cones::ConeLevel::Glow(e),
            },
        );
    }
    e
}

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
        build_mesh(
            commands,
            root,
            parent,
            sco,
            i,
            parts,
            engine.as_deref_mut(),
            &mut rt,
        );
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
