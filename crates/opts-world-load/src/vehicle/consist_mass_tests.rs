//! Tests: consist_mass_tests.

use super::trailers::{manifest_mass_kg, unit_load};
use super::*;

fn bus(mass: Option<f32>, axles: &[bool]) -> BusManifest {
    let mut text = String::new();
    if let Some(m) = mass {
        text.push_str(&format!("[mass]\n{m}\n\n"));
    }
    for driven in axles {
        text.push_str(&format!(
            "[newachse]\n\nachse_antrieb\n{}\n\n",
            u8::from(*driven)
        ));
    }
    BusManifest::parse(&text).expect("manifest")
}

#[test]
fn a_trailer_adds_its_stated_mass_and_driven_axles() {
    let u = unit_load(&bus(Some(7.2), &[false, true]), false, 8_000.0);
    assert_eq!(u.mass_kg, 7_200.0);
    assert_eq!(u.axles.len(), 2);
    assert_eq!(u.axles.iter().filter(|a| a.is_driven).count(), 1);
}

#[test]
fn a_trailer_without_a_mass_takes_the_mass_of_the_unit_in_front() {
    assert_eq!(
        unit_load(&bus(None, &[false]), false, 8_000.0).mass_kg,
        8_000.0
    );
    // An implausible mass counts as missing, like for the head.
    assert_eq!(
        unit_load(&bus(Some(0.1), &[false]), false, 8_000.0).mass_kg,
        8_000.0
    );
}

/// The MAN GN92 (the project's NG272 test vehicle) of the installed corpus: 8 t head and
/// 7.2 t trailer make a 15.2 t consist. Skipped without `OMSI_PATH`.
#[test]
fn the_gn92_consist_weighs_head_and_trailer() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(std::path::PathBuf::from) else {
        return;
    };
    let dir = root.join("Vehicles/MAN_NL_NG");
    let read = |f: &str| {
        std::fs::read(dir.join(f))
            .ok()
            .and_then(|b| BusManifest::parse_bytes(&b).ok())
    };
    let (Some(main), Some(trail)) = (read("MAN_GN92_main.bus"), read("MAN_GN92_trail.bus")) else {
        return;
    };
    let head = manifest_mass_kg(&main).expect("head mass");
    let mut sim = opts_sim::VehicleSimulation::from_manifest(&main);
    sim.set_trailers(vec![unit_load(&trail, false, head)]);
    assert!(
        (sim.consist_mass_kg() - 15_200.0).abs() < 1.0,
        "{}",
        sim.consist_mass_kg()
    );
    assert!((sim.head_mass_kg() - 8_000.0).abs() < 1.0);
}
