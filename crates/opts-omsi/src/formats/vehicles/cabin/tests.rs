//! Unit tests of `cabin`.

use super::*;

const CABIN: &str = "{noticketsale}\n[drivpos]\n-0.72\n4.6\n1.1\n0.4\n0\n\n[passpos]\n0.5\n3.0\n0.4\n0.45\n180\n[passpos]\n-0.5\n2.0\n0.4\n0\n0\n[entry]\n3\n[entry]\n0\n[exit]\n23\n[linkToPrevVeh]\n16\n[stamper]\n5\n0.412\n3.178\n1.741\n[ticket_sale]\n1\n-0.064\n5.075\n1.515\n[ticket_sale_money_point_2]\n-0.06\n5.0\n1.5\n0.02\n0.03\nmesh_x\n[ticket_sale_change_point]\n-0.1\n4.9\n1.5\n0.01\n0.01\n";

#[test]
fn places_doors_machines_and_cash_points_are_read() {
    let c = PassengerCabin::parse(CABIN);
    assert!(c.no_ticket_sale);
    assert_eq!(
        c.driver.expect("driver").position,
        Vec3::new(-0.72, 4.6, 1.1)
    );
    assert_eq!((c.seat_count(), c.standing_count()), (1, 1));
    assert_eq!(c.places[0].rotation_deg, 180.0);
    assert_eq!((c.entries.clone(), c.exits.clone()), (vec![3, 0], vec![23]));
    assert_eq!(c.link_to_prev, Some(16));
    assert_eq!(c.stampers[0].pathpoint, 5);
    assert_eq!(c.ticket_sales[0].position.y, 5.075);
    assert_eq!(c.money_points[0].animparent.as_deref(), Some("mesh_x"));
    assert_eq!(
        c.change_points[0].animparent, None,
        "the legacy variant has none"
    );
}

#[test]
fn the_flag_after_an_entry_belongs_to_that_entry() {
    let c = PassengerCabin::parse("[entry]\n3\n\n[entry]\n39\n\n{noticketsale}\n\n[entry]\n40\n{noticketsale}\n[exit]\n40\n");
    assert!(!c.no_ticket_sale);
    assert_eq!(c.entry_no_sale, vec![false, true, true]);
}

/// Every installed cabin parses with the doors of its paths (`OMSI_PATH`, `--ignored`).
#[test]
#[ignore = "needs an OMSI installation"]
fn every_installed_cabin_parses() {
    let Ok(root) = std::env::var("OMSI_PATH") else {
        return;
    };
    let (mut files, mut places) = (0, 0);
    for v in std::fs::read_dir(std::path::Path::new(&root).join("Vehicles"))
        .expect("Vehicles")
        .flatten()
    {
        for f in std::fs::read_dir(v.path().join("model"))
            .into_iter()
            .flatten()
            .flatten()
        {
            let name = f.file_name().to_string_lossy().to_lowercase();
            if name.starts_with("passengercabin") && name.ends_with(".cfg") {
                let c = PassengerCabin::parse_bytes(&std::fs::read(f.path()).expect("read"));
                files += 1;
                places += c.places.len();
            }
        }
    }
    eprintln!("{files} cabins, {places} places");
    assert!(files > 100);
}
