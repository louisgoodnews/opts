//! The figures an AI profile derives from a vehicle's manifest and constant files: the
//! domain, the wheel dimensions and the speeds.

use super::super::bus::BusManifest;
use super::VehicleDomain;
use crate::formats::constfile::ConstFile;
pub(super) fn constant(files: &[ConstFile], name: &str) -> Option<f32> {
    files
        .iter()
        .rev()
        .flat_map(|f| f.scalars.iter().rev())
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| *v)
}

pub(super) fn determine_domain(m: &BusManifest, notes: &mut Vec<&'static str>) -> VehicleDomain {
    match (m.vehicle_type, m.bogie_half_span) {
        (Some(3), _) => VehicleDomain::Air,
        (Some(2), Some(_)) => VehicleDomain::Rail,
        (Some(2), None) => {
            notes.push("[type] 2 without [boogies]: treated as a road vehicle");
            VehicleDomain::Road
        }
        _ => VehicleDomain::Road,
    }
}

pub(super) fn determine_wheel_dims(m: &BusManifest) -> (Option<f32>, Option<f32>) {
    let wheel_radius_m = m
        .axles
        .iter()
        .find(|a| a.is_driven && a.wheel_diameter_m.is_some_and(|d| d > 0.1))
        .or_else(|| {
            m.axles
                .iter()
                .find(|a| a.wheel_diameter_m.is_some_and(|d| d > 0.1))
        })
        .and_then(|a| a.wheel_diameter_m)
        .map(|d| d * 0.5);
    let wheelbase_m = (m.axles.len() >= 2).then(|| {
        let ys = m.axles.iter().map(|a| a.longitudinal_m);
        let max = ys.clone().fold(f32::NEG_INFINITY, f32::max);
        let min = ys.fold(f32::INFINITY, f32::min);
        max - min
    });
    (wheel_radius_m, wheelbase_m)
}

pub(super) fn determine_speeds(consts: &[ConstFile]) -> (Option<f32>, Option<f32>) {
    let kmh = |v: f32| v / 3.6;
    let script_vmax = constant(consts, "bike_V_max").or_else(|| {
        constant(consts, "vmax_min")
            .map(|v| v + constant(consts, "vmax_add").unwrap_or(0.0).max(0.0) * 0.5)
    });
    let top_speed_ms = constant(consts, "AI_v_max")
        .or(script_vmax)
        .filter(|v| *v > 0.0)
        .map(kmh);
    let reach_speed_ms = constant(consts, "AI_v_max_erreich")
        .filter(|v| *v > 0.0)
        .map(kmh);
    (top_speed_ms, reach_speed_ms)
}
