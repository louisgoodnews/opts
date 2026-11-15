//! Listing session vehicles and resolving one by its name.


use super::*;
use std::path::PathBuf;

use opts_bevy::control::PathPolicy;

use crate::in_game::omsi_world::WorldLoadJob;


pub(super) fn vehicles(world: &mut World, _: &Value) -> Outcome {
    if !world.contains_resource::<SessionVehicles>() {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "no session has been started",
        ));
    }
    let mut out = world.resource::<SessionVehicles>().describe();
    let label = world.get_resource::<PlaceLabel>().and_then(|p| p.0.clone());
    out["place_label"] = json!(label);
    Outcome::ok(out)
}

/// The `.bus` file a `vehicle` parameter names, with the catalogue entry it came from.
pub(super) fn resolve_vehicle(world: &World, text: &str) -> Result<(String, String, PathBuf), RpcError> {
    let as_path = PathBuf::from(text);
    let looks_like_file = text.to_lowercase().ends_with(".bus") || as_path.is_absolute();
    if looks_like_file {
        let checked = world.resource::<PathPolicy>().check_read(&as_path)?;
        if !checked.is_file() {
            return Err(RpcError::new(
                code::NOT_FOUND,
                format!("{} is not a file", checked.display()),
            ));
        }
        let stem = checked
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        return Ok((stem.clone(), stem, checked));
    }
    let item = world
        .get_resource::<GameCatalogue>()
        .and_then(|c| c.find_vehicle(text).cloned())
        .ok_or_else(|| {
            RpcError::new(
                code::NOT_FOUND,
                format!("no vehicle `{text}` in the catalogue (or pass an absolute .bus path)"),
            )
        })?;
    let file = world
        .get_resource::<WorldLoadJob>()
        .and_then(|j| j.bus_file(&item.bus_file))
        .ok_or_else(|| {
            RpcError::new(
                code::NOT_FOUND,
                format!("the .bus file of `{}` was not found", item.full_name),
            )
        })?;
    Ok((item.id, item.full_name, file))
}
