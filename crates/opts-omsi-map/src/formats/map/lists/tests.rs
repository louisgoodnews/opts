//! Unit tests for `super` (`lists.rs`).

use super::*;

#[test]
fn ai_groups_with_and_without_weights() {
    let text = b"[aigroup_2]\r\nNormalCars\r\n\r\nvehicles\\Golf\\AI_Golf.bus\t7\r\nvehicles\\T3\\Van.ovh\r\n[end]\r\n[aigroup_depot]\r\nHof\r\nSpandau\r\n[end]\r\n";
    let lists = AiLists::parse(text);
    assert_eq!(lists.groups.len(), 1);
    let g = lists.group("normalcars").expect("case-insensitive");
    assert_eq!(g.entries.len(), 2);
    assert_eq!(g.entries[0].weight, Some(7.0));
    assert_eq!(g.entries[1].weight, None);
    assert_eq!(g.total_weight(), 8.0);
}

#[test]
fn depot_groups_keep_their_vehicle_file_once() {
    let text = b"[aigroup_depot]\r\nEVAG|NF2\r\nGladbeck\r\n\r\n[aigroup_depot_typgroup_2]\r\nvehicles\\Tram_NF6D\\NF6D_KI.bus\r\n1\t\tEVAG\r\n[end]\r\n[aigroup_depot_typgroup_2]\r\nvehicles\\Tram_NF6D\\nf6d_ki.bus\r\n2\t\tEVAG\r\n[end]\r\n";
    let lists = AiLists::parse(text);
    assert!(lists.groups.is_empty());
    assert_eq!(lists.depot_vehicles, ["vehicles\\Tram_NF6D\\NF6D_KI.bus"]);
    // …but the group keeps every entry, repeats included.
    let g = lists.depot_group("evag|nf2").expect("the depot group");
    assert_eq!(g.depot, "Gladbeck");
    assert_eq!(g.vehicles.len(), 2);
}

#[test]
fn unsched_groups() {
    let g = parse_unsched_vehgroups(b"[group]\nNormalCars\n1\n\n[group]\nBicycles\n0\n");
    assert_eq!(g.len(), 2);
    assert_eq!((g[1].name.as_str(), g[1].default_density), ("Bicycles", 0));
}

#[test]
fn density_curve_with_day_restriction() {
    let text = b"[group]\nNormalCars\n0.6\n\n[set_day_of_week]\n0\n\n[trafficdensity]\n0.000\n0.100\n[trafficdensity]\n4.000\n0.010\n\n[set_day_of_week]\n1\n\n[trafficdensity]\n0.000\n0.100\n[trafficdensity]\n6.000\n1.000\n";
    let d = parse_unsched_trafficdens(text);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].profiles.len(), 2);
    // All-days curve: 0.1 → 0.01 over four hours, times the 0.6 factor.
    assert!((d[0].density(day::SUNDAY, 2.0) - 0.6 * 0.055).abs() < 1e-5);
    // Weekday curve rises to 1.0 at 06:00.
    assert!((d[0].density(day::WEEKDAY, 3.0) - 0.6 * 0.55).abs() < 1e-5);
    assert!((d[0].density(day::WEEKDAY, 12.0) - 0.6).abs() < 1e-5);
}

/// Hours wrap at midnight; weekend curves are picked by the day bit.
#[test]
fn hours_wrap_and_weekends_have_their_own_curve() {
    let text = b"[group]\nNormalCars\n1\n\n[set_day_of_week]\n1\n[trafficdensity]\n0\n0.2\n[trafficdensity]\n12\n1.4\n[trafficdensity]\n24\n0.2\n\n[set_day_of_week]\n6\n[trafficdensity]\n0\n0.1\n[trafficdensity]\n24\n0.5\n";
    let d = &parse_unsched_trafficdens(text)[0];
    let wd = |h| d.density(day::WEEKDAY, h);
    assert!((wd(25.0) - wd(1.0)).abs() < 1e-6 && (wd(-1.0) - wd(23.0)).abs() < 1e-6);
    assert!((wd(36.0) - 1.4).abs() < 1e-6);
    assert!((d.density(day::SATURDAY, 12.0) - 0.3).abs() < 1e-6);
    assert!((d.density(day::SUNDAY, 36.0) - 0.3).abs() < 1e-6);
    // Values above 1 are kept.
    assert!(wd(12.0) > 1.0);
}

#[test]
fn empty_profile_means_baseline() {
    let g = GroupDensity {
        name: "x".into(),
        factor: 2.0,
        profiles: Vec::new(),
    };
    assert_eq!(g.density(day::WEEKDAY, 8.0), 2.0);
}

#[test]
fn the_fleet_lines_of_a_depot_give_number_plate_and_repaint() {
    let text = b"[aigroup_depot]\r\nSolobusse\r\nWinsenburg\r\n\r\n[aigroup_depot_typgroup_2]\r\nVehicles\\MAN_NL_NG\\MAN_EN92_main.bus\r\n101\tWIN-SW 101\tWinsenburg\r\n102\tWIN-Sw 102\tEurobus\t\r\n103\r\n[end]\r\n";
    let lists = AiLists::parse(text);
    let g = &lists.depot_groups[0];
    assert_eq!(g.fleet.len(), 3);
    assert_eq!(g.fleet[0].1.repaint, "Winsenburg");
    assert_eq!(g.fleet[0].1.plate, "WIN-SW 101");
    assert_eq!(g.fleet[1].1.repaint, "Eurobus");
    assert_eq!(
        (g.fleet[2].1.number.as_str(), g.fleet[2].1.repaint.as_str()),
        ("103", "")
    );
    let dated = AiLists::parse(b"[aigroup_depot]\r\nS\r\nD\r\n\r\n[aigroup_depot_typgroup_2]\r\nv.bus\r\n2708\t\tBauer+Syrsch\t19870201\t\r\n2709\t\tX\t19900101\t19951231\r\n[end]\r\n");
    let f = &dated.depot_groups[0].fleet;
    assert_eq!(f[0].1.plate, "");
    assert_eq!(f[0].1.repaint, "Bauer+Syrsch");
    assert!(f[0].1.in_service(19990101) && !f[0].1.in_service(19861231));
    assert!(f[1].1.in_service(19931010) && !f[1].1.in_service(19960101));
    assert!(g
        .fleet
        .iter()
        .all(|(f, _)| f.ends_with("MAN_EN92_main.bus")));
}
