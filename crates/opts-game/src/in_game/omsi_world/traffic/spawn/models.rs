//! Building Bevy model meshes and repaints for traffic vehicle units.

use std::sync::{Arc, RwLock};

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_core::formats::map::FleetEntry;
use opts_core::formats::script_texture::CpuScriptTexture;

use crate::in_game::omsi_world::traffic_variant::Variant;
use crate::in_game::omsi_world::vehicle::load_segment_meshes_for;
use crate::in_game::omsi_world::{trace, ChronoState, WorldLoadJob};
use crate::in_game::types::InGameEntity;

use super::super::variants::unit_lod;

mod repaint;
use repaint::{fleet_repaint_name, head_setvars_of, set_unit_defs};

pub(crate) struct SpawnedModels {
    pub entities: Vec<Entity>,
    pub bounds: Vec<(Vec3, Vec3)>,
    pub origin: Transform,
    pub head_canvases: Option<Arc<RwLock<Vec<CpuScriptTexture>>>>,
    pub head_setvars: Vec<(String, String)>,
    pub trace_frame: trace::Frame,
}

/// One hidden root entity per unit of the vehicle, all at the spawn position.
fn spawn_unit_roots(commands: &mut Commands, variant: &Variant, origin: Transform) -> Vec<Entity> {
    variant
        .units
        .iter()
        .map(|u| {
            commands
                .spawn((
                    origin,
                    Visibility::Hidden,
                    Name::new(u.name.clone()),
                    InGameEntity,
                ))
                .id()
        })
        .collect()
}


pub(crate) fn build_unit_models(
    (commands, job): (&mut Commands, &mut WorldLoadJob),
    (meshes, materials, images): (
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
        &mut Assets<Image>,
    ),
    (variant, at, fleet): (&Variant, DVec2, &[FleetEntry]),
    (repaint_seed, chrono, is_scheduled): (u64, Option<&ChronoState>, bool),
) -> Result<SpawnedModels, bool> {
    let origin = Transform::from_translation(omsi_to_bevy_pos(at.extend(0.0).as_vec3()));
    let entities = spawn_unit_roots(commands, variant, origin);
    let root = entities[0];
    trace::begin_frame();
    let mut bounds = Vec::with_capacity(entities.len());
    let mut failed = None;
    let mut head_canvases = None;
    let fleet_repaint = fleet_repaint_name(fleet, chrono, repaint_seed);
    let pick_repaint = |repaints: &crate::new_game_screen::repaints::VehicleRepaints| -> String {
        fleet_repaint
            .as_deref()
            .and_then(|w| repaints.named(w))
            .unwrap_or_else(|| repaints.pick(repaint_seed))
            .to_owned()
    };
    let head_setvars = head_setvars_of(variant, &pick_repaint);
    for (unit, &entity) in variant.units.iter().zip(&entities) {
        let Some(lod) = unit_lod(unit) else {
            failed = Some(true);
            break;
        };
        if let Some(canvases) = set_unit_defs(job, unit, is_scheduled) {
            if entity == root {
                head_canvases = Some(canvases);
            }
        }
        job.cache.texture_replacements = unit
            .repaints
            .replacements(&pick_repaint(&unit.repaints), &unit.model.ctc_textures)
            .into_iter()
            .collect();
        let (count, _, min, max) = load_segment_meshes_for(
            job,
            commands,
            (&mut *meshes, &mut *materials, &mut *images),
            (entity, root),
            &lod,
            &unit.model_dir.clone(),
            &unit.tex_dirs.clone(),
        );
        job.cache.texture_replacements.clear();
        if count == 0 {
            failed = Some(false);
            break;
        }
        bounds.push((min, max));
    }
    let trace_frame = trace::end_frame();
    if let Some(unusable) = failed {
        for e in entities {
            commands.entity(e).despawn();
        }
        return Err(unusable);
    }
    Ok(SpawnedModels {
        entities,
        bounds,
        origin,
        head_canvases,
        head_setvars,
        trace_frame,
    })
}
