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

pub(crate) struct SpawnedModels {
    pub entities: Vec<Entity>,
    pub bounds: Vec<(Vec3, Vec3)>,
    pub origin: Transform,
    pub head_canvases: Option<Arc<RwLock<Vec<CpuScriptTexture>>>>,
    pub head_setvars: Vec<(String, String)>,
    pub trace_frame: trace::Frame,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_unit_models(
    commands: &mut Commands,
    job: &mut WorldLoadJob,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    variant: &Variant,
    at: DVec2,
    fleet: &[FleetEntry],
    repaint_seed: u64,
    chrono: Option<&ChronoState>,
    is_scheduled: bool,
) -> Result<SpawnedModels, bool> {
    let origin = Transform::from_translation(omsi_to_bevy_pos(at.extend(0.0).as_vec3()));
    let entities: Vec<Entity> = variant
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
        .collect();
    let root = entities[0];
    trace::begin_frame();
    let mut bounds = Vec::with_capacity(entities.len());
    let mut failed = None;
    let mut head_canvases = None;
    let fleet_repaint = {
        let today = chrono
            .and_then(|c| c.date)
            .map(opts_core::formats::chrono::ChronoDate::yyyymmdd);
        let in_service: Vec<_> = fleet
            .iter()
            .filter(|e| today.is_none_or(|d| e.in_service(d)))
            .collect();
        let fleet = if in_service.is_empty() {
            fleet.iter().collect()
        } else {
            in_service
        };
        (!fleet.is_empty())
            .then(|| fleet[usize::try_from(repaint_seed % fleet.len() as u64).unwrap_or(0)])
            .map(|e| e.repaint.clone())
    };
    let pick_repaint = |repaints: &crate::new_game_screen::repaints::VehicleRepaints| -> String {
        fleet_repaint
            .as_deref()
            .and_then(|w| repaints.named(w))
            .unwrap_or_else(|| repaints.pick(repaint_seed))
            .to_owned()
    };
    let head_setvars = {
        let repaints = &variant.units[0].repaints;
        let picked = pick_repaint(repaints);
        let picked = picked.as_str();
        let name = if picked.is_empty() {
            repaints.default_name.as_str()
        } else {
            picked
        };
        repaints.setvars.get(name).cloned().unwrap_or_default()
    };
    for (unit, &entity) in variant.units.iter().zip(&entities) {
        let Some(lod) = unit_lod(unit) else {
            failed = Some(true);
            break;
        };
        job.vehicle_text_defs = unit.model.text_textures.clone();
        if is_scheduled {
            job.vehicle_script_defs = unit.model.script_textures.clone();
            let canvases = unit
                .model
                .script_textures
                .iter()
                .map(|st| CpuScriptTexture::new(st.width, st.height))
                .collect();
            job.vehicle_script_textures = Arc::new(RwLock::new(canvases));
            if entity == root {
                head_canvases = Some(job.vehicle_script_textures.clone());
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
            meshes,
            materials,
            images,
            entity,
            root,
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
