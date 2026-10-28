//! Unit tests for `super` (`traffic_variant.rs`).

use super::*;
use opts_core::formats::map::{AiLists, UnschedGroup};

fn lists() -> AiLists {
    AiLists::parse(
        b"[aigroup_2]\nNormalCars\n\nvehicles\\a.bus\t5\n[end]\n\n[aigroup_2]\nTrucks\n\nvehicles\\b.bus\t1\n[end]\n\n\
[aigroup_2]\nEmergency\n\nvehicles\\c.bus\t1\n[end]\n",
    )
}

fn vg(name: &str, d: u32) -> UnschedGroup {
    UnschedGroup {
        name: name.into(),
        default_density: d,
    }
}

/// Every listed group takes part with its index in `unsched_vehgroups.txt` (the rule flag),
/// also those with default `0`; groups `ailists.cfg` lacks are skipped.
#[test]
fn listed_groups_take_part_with_their_rule_index() {
    let l = lists();
    let g = participating(
        &l,
        &[
            vg("NormalCars", 1),
            vg("Taxi", 1),
            vg("trucks", 0),
            vg("Emergency", 0),
        ],
    );
    let names: Vec<(usize, &str)> = g.iter().map(|(i, g)| (*i, g.name.as_str())).collect();
    assert_eq!(names, [(0, "NormalCars"), (2, "Trucks"), (3, "Emergency")]);
}

#[test]
fn without_usable_group_data_the_old_choice_stays() {
    let l = lists();
    assert_eq!(participating(&l, &[])[0].1.name, "NormalCars");
    assert_eq!(
        participating(&l, &[vg("Bicycles", 0)])[0],
        (0, &l.groups[0])
    );
    let only = AiLists::parse(b"[aigroup_2]\nTaxi\n\nvehicles\\t.bus\t1\n[end]\n");
    assert_eq!(participating(&only, &[])[0].1.name, "Taxi");
    assert!(participating(&AiLists::default(), &[]).is_empty());
}

#[test]
fn traits_sum_the_units_of_a_coupled_vehicle() {
    let unit = |mass: f32, brake: f32, len: f32| Unit {
        bus_path: PathBuf::new(),
        name: "u".into(),
        model: ModelConfig::default(),
        model_dir: PathBuf::new(),
        tex_dirs: vec![],
        response: None,
        profile: {
            let mut p = AiProfile::from_manifest(&BusManifest::parse("[mass]\n1\n").unwrap(), &[]);
            p.mass_kg = Some(mass);
            p.brake_max_n = Some(brake);
            p
        },
        size: Some((len, 1.0)),
        hitch: Vec2::ZERO,
        joint: Vec2::ZERO,
        rear_axle_y: 0.0,
        front_axle_y: None,
        max_alpha_deg: 52.5,
        repaints: Default::default(),
        reversed: false,
        sound: None,
        front_door_y: None,
    };
    let v = Variant {
        name: "bus".into(),
        units: vec![unit(11_000.0, 80_000.0, 11.0), unit(8000.0, 60_000.0, 7.0)],
    };
    let t = v.traits();
    assert_eq!(t.mass_kg, Some(19_000.0));
    assert_eq!(t.brake_force, Some(140_000.0));
    assert!((t.length - 18.0).abs() < 1e-9);
    assert_eq!(
        v.class(),
        VehicleClass::Truck,
        "no kind and no cabin: by length"
    );
}
