//! Unit tests of `humans`.

use super::*;

const SAMPLE: &str = "[model]\nmodel\\man.cfg\n\n[seatheight]\n0.81\n\n\t[humangeom]\n\t{feetdist (FD)}\n\t{height}\n\n\t[links]\n\t{x_hip}\n\n[humangeom]\n0.04\n1.77\n\n[links]\n0.09\n0.00\n0.92\n0.09\n-0.03\n0.53\n0.02\n1.17\n0.18\n-0.05\n1.43\n0.44\n-0.04\n1.41\n-0.02\n1.55\n0.69\n-0.03\n1.43\n0.90\n-0.03\n1.43\n\n[walk_param]\n1.4\n70\n0.7\n1.0\n0.0\n\n[voice]\nM2\n\n[mass]\n0.07\n\n[age]\n12\n";

#[test]
fn parses_template_commented_profile() {
    let h = HumanProfile::from_text(SAMPLE).expect("parse");
    assert_eq!(h.model, "model\\man.cfg");
    assert_eq!(
        (h.seat_height_m, h.feet_dist_m, h.height_m),
        (0.81, 0.04, 1.77)
    );
    assert_eq!(h.links.hip, Vec3::new(0.09, 0.0, 0.92));
    assert_eq!(h.links.neck, Vec2::new(-0.02, 1.55));
    assert_eq!(h.links.finger, Vec3::new(0.9, -0.03, 1.43));
    assert_eq!(h.walk.stride_m, 1.4);
    assert_eq!(
        (h.voice.as_str(), h.mass_tonnes, h.age_years),
        ("M2", 0.07, Some(12))
    );
    assert!((h.eye_position().z - 1.66).abs() < 1e-5);
}

#[test]
fn reports_missing_and_malformed_fields() {
    assert_eq!(
        HumanProfile::from_text("[model]\na.cfg\n"),
        Err(HumanParseError::MissingKeyword("seatheight"))
    );
    let broken = SAMPLE.replace("0.81", "tall");
    assert!(matches!(
        HumanProfile::from_text(&broken),
        Err(HumanParseError::InvalidNumber {
            keyword: "seatheight",
            ..
        })
    ));
    let short = SAMPLE.replace("[mass]\n0.07\n", "[mass]\n");
    assert!(matches!(
        HumanProfile::from_text(&short),
        Err(HumanParseError::TooFewParameters {
            keyword: "mass",
            ..
        })
    ));
}
