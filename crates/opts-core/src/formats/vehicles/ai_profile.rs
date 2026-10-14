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

impl AiProfile {
    /// Builds the profile of a vehicle from its manifest and the constant files it lists (in
    /// the manifest's order; a later file overrides an earlier one).
    #[must_use]
    pub fn from_manifest(m: &BusManifest, consts: &[ConstFile]) -> Self {
        let mut fallbacks = Vec::new();
        let mut notes = Vec::new();
        let kmh = |v: f32| v / 3.6;
        let mass_kg = m.mass_tonnes.filter(|t| *t > 0.0).map(|t| t * 1000.0);
        let top_speed_ms = constant(consts, "AI_v_max").filter(|v| *v > 0.0).map(kmh);
        let reach_speed_ms = constant(consts, "AI_v_max_erreich")
            .filter(|v| *v > 0.0)
            .map(kmh);
        let gas_max = constant(consts, "AI_M_gas_max").filter(|v| *v > 0.0);
        let brake_max_n = constant(consts, "AI_F_brems_max").filter(|v| *v > 0.0);
        let motor_curve = consts
            .iter()
            .rev()
            .flat_map(|f| f.curves.iter())
            .find(|c| c.name.eq_ignore_ascii_case("AI_Motorkennlinie"))
            .map(|c| c.points.clone())
            .unwrap_or_default();
        let domain = match (m.vehicle_type, m.bogie_half_span) {
            (Some(3), _) => VehicleDomain::Air,
            (Some(2), Some(_)) => VehicleDomain::Rail,
            (Some(2), None) => {
                notes.push("[type] 2 without [boogies]: treated as a road vehicle");
                VehicleDomain::Road
            }
            _ => VehicleDomain::Road,
        };
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
mod tests {
    use super::*;

    const CAR: &str =
        "[mass]\n1.3\n0\n0\n0\n\n[type]\n0\n\n[ai_veh_type]\n3\n\n[rowdy_factor]\n0.2\n0.8\n\n\
[ai_brakeperformance]\n1.3\n0.0\n\n[ai_deltaheight]\n-0,10\n\n[rollwiderstand]\n150\n\n\
[inv_min_turnradius]\n0.2\n\n[schwerpunkt]\n0.55\n\n[rot_pnt_long]\n-1.2\n\n\
[newachse]\nachse_long\n1.2\nachse_raddurchmesser\n0.62\n\n\
[newachse]\nachse_long\n-1.4\nachse_raddurchmesser\n0.66\nachse_antrieb\n1\n\n\
[boundingbox]\n1.7\n4.1\n1.4\n0\n0\n0\n0\n0\n0\n0\n0\n0\n";
    const CONST: &str = "[const]\nAI_v_max\n160\n\n[const]\nAI_v_max_erreich\n190\n\n[const]\nAI_M_gas_max\n1000\n\n\
[const]\nAI_F_brems_max\n18000\n\n[newcurve]\nAI_Motorkennlinie\n\n[pnt]\n0\n0.3\n\n[pnt]\n15\n1\n";

    fn manifest(text: &str) -> BusManifest {
        BusManifest::parse(text).expect("parse")
    }

    #[test]
    fn a_complete_car_has_no_fallbacks_and_all_values() {
        let p = AiProfile::from_manifest(&manifest(CAR), &[ConstFile::parse(CONST.as_bytes())]);
        assert_eq!(p.mass_kg, Some(1300.0));
        assert_eq!(p.kind, Some(AiKind::Truck));
        assert_eq!(p.domain, VehicleDomain::Road);
        assert!((p.top_speed_ms.unwrap() - 160.0 / 3.6).abs() < 1e-4);
        assert_eq!((p.gas_max, p.brake_max_n), (Some(1000.0), Some(18000.0)));
        assert_eq!(p.motor_curve, vec![(0.0, 0.3), (15.0, 1.0)]);
        assert_eq!(p.rowdy, (0.2, 0.8));
        assert!((p.brake_performance - 1.3).abs() < 1e-6);
        assert!((p.delta_height_m + 0.10).abs() < 1e-6);
        assert_eq!(p.body_length_m, Some(4.1));
        assert!((p.wheelbase_m.unwrap() - 2.6).abs() < 1e-5);
        assert!(
            (p.wheel_radius_m.unwrap() - 0.33).abs() < 1e-5,
            "the driven axle"
        );
        assert!(p.fallbacks.is_empty(), "{:?}", p.fallbacks);
    }

    #[test]
    fn missing_values_are_listed_as_fallbacks_and_defaults_are_neutral() {
        let p = AiProfile::from_manifest(&manifest("[type]\n0\n"), &[]);
        assert_eq!(
            p.fallbacks,
            ["mass", "top_speed", "gas_max", "brake_max", "wheel_radius"]
        );
        assert_eq!((p.brake_performance, p.rowdy), (1.0, (0.0, 1.0)));
        assert_eq!(p.kind, None);
        assert_eq!(p.delta_height_m, 0.0);
    }

    #[test]
    fn a_later_constfile_overrides_an_earlier_one() {
        let a = ConstFile::parse(b"[const]\nAI_v_max\n100\n");
        let b = ConstFile::parse(b"[const]\nai_V_MAX\n50\n");
        let p = AiProfile::from_manifest(&manifest("[mass]\n1\n"), &[a, b]);
        assert!((p.top_speed_ms.unwrap() - 50.0 / 3.6).abs() < 1e-4);
    }

    #[test]
    fn domain_follows_type_and_bogies() {
        let rail = manifest("[type]\n2\n\n[boogies]\n5.0\n");
        assert_eq!(
            AiProfile::from_manifest(&rail, &[]).domain,
            VehicleDomain::Rail
        );
        let scooter = AiProfile::from_manifest(&manifest("[type]\n2\n"), &[]);
        assert_eq!(scooter.domain, VehicleDomain::Road);
        assert_eq!(scooter.notes.len(), 1);
        let air = manifest("[type]\n3\n");
        assert_eq!(
            AiProfile::from_manifest(&air, &[]).domain,
            VehicleDomain::Air
        );
    }

    #[test]
    fn kinds_follow_the_legend() {
        assert_eq!(AiKind::from_value(0), Some(AiKind::Car));
        assert_eq!(AiKind::from_value(1), Some(AiKind::Cab));
        assert_eq!(AiKind::from_value(2), Some(AiKind::Bus));
        assert_eq!(AiKind::from_value(3), Some(AiKind::Truck));
        assert_eq!(AiKind::from_value(7), None);
    }

    #[test]
    fn the_coupling_and_cabin_are_carried_over() {
        let m = manifest("[couple_back]\nTrailer.bus\n1\n\n[passengercabin]\ncab.cfg\n");
        let p = AiProfile::from_manifest(&m, &[]);
        assert_eq!(p.couple_back.as_deref(), Some("Trailer.bus"));
        assert!(p.has_cabin);
    }
}
