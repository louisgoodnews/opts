//! Tests for AI vehicle profile mapping.

use super::*;

const CAR: &str =
    "[mass]\n1.3\n0\n0\n0\n\n[type]\n0\n\n[ai_veh_type]\n3\n\n[rowdy_factor]\n0.2\n0.8\n\n\
[ai_brakeperformance]\n1.3\n0.0\n\n[ai_deltaheight]\n-0,10\n\n[rollwiderstand]\n150\n\n\
[inv_min_turnradius]\n0.2\n\n[schwerpunkt]\n0.55\n\n[rot_pnt_long]\n-1.2\n\n\
[newachse]\nachse_long\n1.2\nachse_raddurchmesser\n0.62\n\n\
[newachse]\nachse_long\n-1.4\nachse_raddurchmesser\n0.66\nachse_antrieb\n1\n\n\
[boundingbox]\n1.7\n4.1\n1.4\n0\n0\n0\n0\n0\n0\n0\n0\n0\n";
const CONST: &str =
    "[const]\nAI_v_max\n160\n\n[const]\nAI_v_max_erreich\n190\n\n[const]\nAI_M_gas_max\n1000\n\n\
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
fn a_bicycle_rides_at_its_script_speed() {
    let th = ConstFile::parse(b"[const]\nbike_V_max\n20\n");
    let p = AiProfile::from_manifest(&manifest("[mass]\n0.1\n"), &[th]);
    assert!((p.top_speed_ms.unwrap() - 20.0 / 3.6).abs() < 1e-4);
    let hc = ConstFile::parse(b"[const]\nvmax_min\n15\n\n[const]\nvmax_add\n6\n");
    let p = AiProfile::from_manifest(&manifest("[mass]\n0.1\n"), &[hc]);
    assert!((p.top_speed_ms.unwrap() - 18.0 / 3.6).abs() < 1e-4);
    let both = ConstFile::parse(b"[const]\nAI_v_max\n15\n\n[const]\nvmax_min\n30\n");
    let p = AiProfile::from_manifest(&manifest("[mass]\n0.1\n"), &[both]);
    assert!(
        (p.top_speed_ms.unwrap() - 15.0 / 3.6).abs() < 1e-4,
        "AI_v_max wins"
    );
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
