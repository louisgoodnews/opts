//! Unit tests of `light`.

use super::*;

fn block(keyword: &str, lines: &[&str]) -> OmsiBlock {
    OmsiBlock {
        keyword: keyword.into(),
        line_number: 1,
        parameters: lines.iter().map(|s| (*s).to_owned()).collect(),
    }
}

#[test]
fn light_enh_2_reads_position_colour_cone_and_variable() {
    let b = block(
        "light_enh_2",
        &[
            "0", "-0.1", "0.3", "0", "2", "0", "0", "0", "1", "0", "0", "255", "10", "0", "0.25",
            "20", "50", "lamp_red", "1.5", "0.1", "1", "0", "0.1",
        ],
    );
    let l = parse_light_enh_2(&b).unwrap();
    assert_eq!(l.position, Vec3::new(0.0, -0.1, 0.3));
    assert_eq!(l.direction, Vec3::Y);
    assert!(!l.omni);
    assert_eq!(l.color, [255.0, 10.0, 0.0]);
    assert_eq!(l.size_m, 0.25);
    assert_eq!((l.cone_inner_deg, l.cone_outer_deg), (20.0, 50.0));
    assert_eq!(l.variable, "lamp_red");
    assert_eq!(l.factor, 1.5);
    assert_eq!(l.time_const_s, 0.1);
    assert!(l.bitmap.is_empty());
    assert!(!l.volumetric_cone);
}

/// Parameter 22 (`cone` = 1) marks a light that draws a volumetric cone; the rest do not.
#[test]
fn light_enh_2_reads_the_volumetric_cone_flag() {
    let mut v: Vec<String> = [
        "0",
        "0",
        "7",
        "0",
        "0",
        "-1",
        "0",
        "1",
        "0",
        "0",
        "1",
        "230",
        "230",
        "255",
        "2",
        "120",
        "200",
        "NightlightA",
        "0.8",
        "0.5",
        "1",
        "1",
        "0.2",
    ]
    .map(String::from)
    .to_vec();
    let mk = |v: &[String]| OmsiBlock {
        keyword: "light_enh_2".into(),
        line_number: 1,
        parameters: v.to_vec(),
    };
    assert!(parse_light_enh_2(&mk(&v)).unwrap().volumetric_cone);
    v[21] = "0".into();
    assert!(!parse_light_enh_2(&mk(&v)).unwrap().volumetric_cone);
    v[21] = "2".into();
    assert!(!parse_light_enh_2(&mk(&v)).unwrap().volumetric_cone);
}

#[test]
fn light_enh_reads_the_short_layout() {
    let b = block(
        "light_enh",
        &[
            "1", "2", "3", "255", "255", "200", "0.5", "night", "0", "0", "0", "0",
        ],
    );
    let l = parse_light_enh(&b).unwrap();
    assert_eq!(l.position, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(l.variable, "night");
    assert!(l.omni);
}

#[test]
fn malformed_lights_are_errors_not_panics() {
    assert!(parse_light_enh_2(&block("light_enh_2", &["x"])).is_err());
    assert!(parse_light_enh(&block("light_enh", &[])).is_err());
}
