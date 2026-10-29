//! What a vehicle file says about how an AI vehicle drives: the hand-over from `.bus`/`.ovh` and
//! its constant files to the traffic model (`docs/src/architecture/ai-vehicle-config.md`).
//!
//! The profile only gathers data; turning it into driving parameters is `opts-traffic`'s job.
//! Every field that the mapping needs and the files do not give is listed in
//! [`AiProfile::fallbacks`], so a corpus test can say which vehicles run on defaults.

use super::bus::BusManifest;
use crate::formats::constfile::ConstFile;

/// The simulation domain a vehicle belongs to (`[type]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleDomain {
    /// A road vehicle.
    Road,
    /// A rail-bound vehicle (`[type]` 2 with `[boogies]`).
    Rail,
    /// An aircraft or other airborne object (`[type]` 3).
    Air,
}

/// The kind of AI vehicle, from the legend of `[ai_veh_type]` in the vehicle files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiKind {
    /// `0`: a normal passenger car.
    Car,
    /// `1`: a cab.
    Cab,
    /// `2`: a bus.
    Bus,
    /// `3`: a truck.
    Truck,
}

impl AiKind {
    /// The kind for an `[ai_veh_type]` value.
    #[must_use]
    pub const fn from_value(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::Car),
            1 => Some(Self::Cab),
            2 => Some(Self::Bus),
            3 => Some(Self::Truck),
            _ => None,
        }
    }
}

/// Everything the AI model may use from one vehicle file. `None` means "not declared".
#[derive(Debug, Clone, PartialEq)]
pub struct AiProfile {
    /// `[mass]` in kilograms.
    pub mass_kg: Option<f32>,
    /// `[type]` and `[boogies]`.
    pub domain: VehicleDomain,
    /// `[ai_veh_type]`.
    pub kind: Option<AiKind>,
    /// `AI_v_max` in m/s.
    pub top_speed_ms: Option<f32>,
    /// `AI_v_max_erreich` in m/s: the speed at which the AI script's drive torque has fallen to
    /// zero.
    pub reach_speed_ms: Option<f32>,
    /// `AI_M_gas_max`: drive torque at full throttle (the AI script's `M_Wheel` scale).
    pub gas_max: Option<f32>,
    /// `AI_F_brems_max`: brake force at full brake, newtons.
    pub brake_max_n: Option<f32>,
    /// `AI_Motorkennlinie`: factor over speed in km/h, in file order.
    pub motor_curve: Vec<(f32, f32)>,
    /// First number of `[ai_brakeperformance]`; `1.0` when absent.
    pub brake_performance: f32,
    /// `[rowdy_factor]` range; `(0, 1)` when absent.
    pub rowdy: (f32, f32),
    /// `[boundingbox]` length, metres.
    pub body_length_m: Option<f32>,
    /// `[boundingbox]` width, metres.
    pub body_width_m: Option<f32>,
    /// Distance between the first and last `[newachse]`, metres.
    pub wheelbase_m: Option<f32>,
    /// Radius of the driven axle's wheels (else the first axle's), metres.
    pub wheel_radius_m: Option<f32>,
    /// `[couple_back]`: the trailing unit's file, relative to this vehicle's folder.
    pub couple_back: Option<String>,
    /// The trailing unit is coupled the other way round.
    pub couple_back_reversed: bool,
    /// `[ai_deltaheight]`, metres; `0` when absent.
    pub delta_height_m: f32,
    /// The vehicle has a passenger cabin (`[passengercabin]`).
    pub has_cabin: bool,
    /// Names of the fields the mapping needs that the files did not give.
    pub fallbacks: Vec<&'static str>,
    /// Oddities worth a log line (for example `[type] 2` without bogies).
    pub notes: Vec<&'static str>,
}

fn constant(files: &[ConstFile], name: &str) -> Option<f32> {
    files
        .iter()
        .rev()
        .flat_map(|f| f.scalars.iter().rev())
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| *v)
}

fn determine_domain(m: &BusManifest, notes: &mut Vec<&'static str>) -> VehicleDomain {
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

fn determine_wheel_dims(m: &BusManifest) -> (Option<f32>, Option<f32>) {
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

fn determine_speeds(consts: &[ConstFile]) -> (Option<f32>, Option<f32>) {
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

impl AiProfile {
    /// Builds the profile of a vehicle from its manifest and the constant files it lists (in
    /// the manifest's order; a later file overrides an earlier one).
    #[must_use]
    pub fn from_manifest(m: &BusManifest, consts: &[ConstFile]) -> Self {
        let mut fallbacks = Vec::new();
        let mut notes = Vec::new();
        let mass_kg = m.mass_tonnes.filter(|t| *t > 0.0).map(|t| t * 1000.0);
        let (top_speed_ms, reach_speed_ms) = determine_speeds(consts);
        let gas_max = constant(consts, "AI_M_gas_max").filter(|v| *v > 0.0);
        let brake_max_n = constant(consts, "AI_F_brems_max").filter(|v| *v > 0.0);
        let motor_curve = consts
            .iter()
            .rev()
            .flat_map(|f| f.curves.iter())
            .find(|c| c.name.eq_ignore_ascii_case("AI_Motorkennlinie"))
            .map(|c| c.points.clone())
            .unwrap_or_default();
        let domain = determine_domain(m, &mut notes);
        let (wheel_radius_m, wheelbase_m) = determine_wheel_dims(m);
        for (name, missing) in [
            ("mass", mass_kg.is_none()),
            ("top_speed", top_speed_ms.is_none()),
            ("gas_max", gas_max.is_none()),
            ("brake_max", brake_max_n.is_none()),
            ("wheel_radius", wheel_radius_m.is_none()),
        ] {
            if missing {
                fallbacks.push(name);
            }
        }
        let x = &m.extras;
        Self {
            mass_kg,
            domain,
            kind: x.ai_veh_type.and_then(AiKind::from_value),
            top_speed_ms,
            reach_speed_ms,
            gas_max,
            brake_max_n,
            motor_curve,
            brake_performance: x
                .ai_brake_performance
                .first()
                .copied()
                .filter(|v| *v > 0.0)
                .unwrap_or(1.0),
            rowdy: x.rowdy_factor.unwrap_or((0.0, 1.0)),
            body_length_m: m.bounding_box.map(|(size, _)| size.y).filter(|l| *l > 0.0),
            body_width_m: m.bounding_box.map(|(size, _)| size.x).filter(|w| *w > 0.0),
            wheelbase_m,
            wheel_radius_m,
            couple_back: m.couple_back.clone(),
            couple_back_reversed: m.couple_back_reversed,
            delta_height_m: x.ai_delta_height.unwrap_or(0.0),
            has_cabin: m.passengercabin_cfg.is_some(),
            fallbacks,
            notes,
        }
    }
}

#[cfg(test)]
mod tests;
