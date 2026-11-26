//! Unit tests of the covered-section rule: a ceiling of 15 m or more gives a zone, an overhead
//! wire gives none, and a short covered section gives none.

use bevy::math::Vec2;

use opts_spatial_zones::ceiling::*;
use opts_spatial_zones::model::{ZoneId, ZoneKind};

const ROOF: &str = "Splines/Ruede/rail_wood_S49_tunnel.sli";
const WIRE: &str = "Splines/ADDON_Gladbeck_NF6D/Meterspur_normal_OL_408.sli";

/// One graphical profile: a flat strip at height `z` from `-3` to `3` m.
fn profile_at(z: f32) -> Vec<Vec<Vec2>> {
    vec![vec![Vec2::new(-3.0, z), Vec2::new(3.0, z)]]
}

fn candidate<'a>(sli: &'a str, profiles: &'a [Vec<Vec2>], length_m: f32) -> SplineCandidate<'a> {
    SplineCandidate {
        sli_file: sli,
        profiles,
        start: Vec2::new(10.0, 20.0),
        end: Vec2::new(10.0, 20.0 + length_m),
        length_m,
    }
}

#[test]
fn a_covered_spline_of_15_m_or_more_yields_a_zone_padded_by_the_ceiling() {
    let roof = profile_at(6.0);
    let zone = covered_zone(ZoneId(7), &candidate(ROOF, &roof, 15.0)).expect("zone");
    assert_eq!(zone.id, ZoneId(7));
    assert_eq!(zone.kind, ZoneKind::Tunnel);
    assert_eq!(zone.min, Vec2::new(7.0, 17.0));
    assert_eq!(zone.max, Vec2::new(13.0, 38.0));
}

/// `str_6m_Tunnel.sli` (Bad Hügelsdorf): walls at ground level, a roof run at 4.6 m, 3.2 m half-width.
fn u_profile() -> Vec<Vec<Vec2>> {
    vec![vec![
        Vec2::new(0.0, 4.6),
        Vec2::new(-1.5, 4.6),
        Vec2::new(-2.2, 4.4),
        Vec2::new(-3.2, 3.8),
        Vec2::new(-3.2, 0.1),
    ]]
}

#[test]
fn a_u_profile_is_a_ceiling_through_its_roof_run() {
    let u = u_profile();
    assert_eq!(ceiling_half_width(&u), Some(3.2));
    let zone = covered_zone(ZoneId(0), &candidate(ROOF, &u, 20.0)).expect("zone");
    assert_eq!(zone.max, Vec2::new(13.2, 43.2));
}

#[test]
fn an_overhead_wire_profile_yields_no_zone() {
    let wire = profile_at(4.57);
    assert!(covered_zone(ZoneId(0), &candidate(WIRE, &wire, 80.0)).is_none());
    // The height test alone would pass a wire at 5.5 m; the name excludes it.
    let high_wire = profile_at(5.5);
    assert!(covered_zone(ZoneId(0), &candidate(WIRE, &high_wire, 80.0)).is_none());
}

#[test]
fn a_short_covered_section_yields_no_zone() {
    let roof = profile_at(6.0);
    assert!(covered_zone(ZoneId(0), &candidate(ROOF, &roof, 10.0)).is_none());
}

#[test]
fn a_spline_without_a_ceiling_yields_no_zone() {
    let open = profile_at(0.1);
    assert!(covered_zone(ZoneId(0), &candidate(ROOF, &open, 40.0)).is_none());
}

#[test]
fn covered_zones_numbers_only_the_covered_splines_in_order() {
    let roof = profile_at(6.0);
    let open = profile_at(0.1);
    let wire = profile_at(4.57);
    let splines = [
        candidate(ROOF, &open, 40.0),
        candidate(ROOF, &roof, 20.0),
        candidate(WIRE, &wire, 80.0),
        candidate(ROOF, &roof, 30.0),
    ];
    let zones = covered_zones(&splines);
    let ids: Vec<ZoneId> = zones.iter().map(|z| z.id).collect();
    assert_eq!(ids, vec![ZoneId(0), ZoneId(1)]);
}

#[test]
fn overhead_wire_names_are_recognised_with_either_separator() {
    assert!(is_overhead_wire(
        "Splines/x/Meterspur_normal_OL_408_Streckentrenner.sli"
    ));
    assert!(is_overhead_wire("Splines\\x\\Oberleitung_Mast.sli"));
    assert!(!is_overhead_wire(ROOF));
}
