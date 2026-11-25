//! The repaint a spawn wears (the fleet entry the seed picks, else the head's default) and the
//! unit definitions it installs for the mesh loader.

use super::*;

/// The repaint the fleet entry of this spawn names: among the entries in service on the map date
/// (all when none is), the one the seed picks.
pub(super) fn fleet_repaint_name(
    fleet: &[FleetEntry],
    chrono: Option<&ChronoState>,
    repaint_seed: u64,
) -> Option<String> {
    {
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
    }
}

/// Makes the unit's text and script texture definitions the ones the mesh loader sees; for a
/// scheduled vehicle also a fresh set of canvases, which are returned.
pub(super) fn set_unit_defs(
    job: &mut WorldLoadJob,
    unit: &crate::traffic_variant::Unit,
    is_scheduled: bool,
) -> Option<Arc<RwLock<Vec<CpuScriptTexture>>>> {
    job.vehicle_text_defs = unit.model.text_textures.clone();
    if !is_scheduled {
        return None;
    }
    job.vehicle_script_defs = unit.model.script_textures.clone();
    let canvases = unit
        .model
        .script_textures
        .iter()
        .map(|st| CpuScriptTexture::new(st.width, st.height))
        .collect();
    job.vehicle_script_textures = Arc::new(RwLock::new(canvases));
    Some(job.vehicle_script_textures.clone())
}

/// The equipment options of the repaint the head unit gets.
pub(super) fn head_setvars_of(
    variant: &Variant,
    pick_repaint: &dyn Fn(&opts_repaints::VehicleRepaints) -> String,
) -> Vec<(String, String)> {
    let repaints = &variant.units[0].repaints;
    let picked = pick_repaint(repaints);
    let picked = picked.as_str();
    let name = if picked.is_empty() {
        repaints.default_name.as_str()
    } else {
        picked
    };
    repaints.setvars.get(name).cloned().unwrap_or_default()
}
