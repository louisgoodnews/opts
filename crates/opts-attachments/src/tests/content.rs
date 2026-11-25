//! The corpus checks of attachments: the parking bays of a real map and the repeaters across it.

use super::super::*;
use super::corpus::{index_of, omsi, tiles};

/// Berlin-Spandau tile 2404/11277: the parking bays of every run stand exactly `spacing`
/// (7 m) apart along the chain, none outside `[d, d + range]`; map-wide counts are printed.
#[test]
fn spandau_parking_bays_follow_spacing_and_range() {
    let Some(root) = omsi() else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let map = root.join("maps").join("Berlin-Spandau");
    if !map.is_dir() {
        eprintln!("Berlin-Spandau not installed; skipping");
        return;
    }
    let tiles = tiles(&map);
    let index = index_of(&tiles);
    let (_, tile) = tiles
        .iter()
        .find(|(k, _)| *k == (2404, 11277))
        .expect("tile 2404/11277");
    let mut runs = 0;
    let mut bays = 0;
    for att in tile
        .spline_attachments
        .iter()
        .filter(|a| a.sco_file.to_ascii_lowercase().contains("car_park") && a.is_run())
    {
        let carrier = &tile.splines[att.spline_index];
        let sites = instance_sites(att, carrier, &index);
        assert!(
            (att.spacing_m - 7.0).abs() < 1e-3,
            "spacing {}",
            att.spacing_m
        );
        for s in &sites {
            let d = f64::from(att.distance_m);
            assert!(
                s.chainage >= d - 1e-6 && s.chainage <= d + f64::from(att.range_m) + 1e-3,
                "bay outside [d, d + range]: {} not in [{d}, {}]",
                s.chainage,
                d + f64::from(att.range_m)
            );
            assert!(s.local >= 0.0 && s.local <= f64::from(s.spline.1.length_m) + 1e-3);
        }
        for w in sites.windows(2) {
            assert_eq!(w[1].k, w[0].k + 1);
            assert!((w[1].chainage - w[0].chainage - 7.0).abs() < 1e-3);
        }
        runs += 1;
        bays += sites.len();
    }
    eprintln!("tile 2404/11277: {runs} parking runs, {bays} bays");
    assert!(runs > 0 && bays > runs, "{runs} runs, {bays} bays");

    // Map-wide: instances placed per kind (bays, lamps).
    let count = |needle: &str| -> usize {
        tiles
            .iter()
            .map(|(key, t)| {
                let atts: Vec<MapSplineAttachment> = t
                    .spline_attachments
                    .iter()
                    .filter(|a| a.sco_file.to_ascii_lowercase().contains(needle))
                    .cloned()
                    .collect();
                expand_attachments(
                    &atts,
                    &t.splines,
                    &index,
                    *key,
                    &TileGrid::default(),
                    |_, _| 0.0,
                )
                .len()
            })
            .sum()
    };
    let (cars, lights) = (count("car_park"), count("light"));
    eprintln!("Berlin-Spandau: {cars} parking bays, {lights} 'light' attachments placed");
    // The probe model gives 3,858 instances, 84 of them from 3 repeaters that repeat others.
    assert_eq!(cars, 3774, "parking bays");
    assert_eq!(lights, 1806, "'light' attachments");
}

/// Corpus-wide: a repeater's first instance `p2` lies on its own spline and instance `p2 − 1`
/// does not, for at least 85 % of the repeaters (measured 87.7 %).
#[test]
fn repeater_first_instance_is_consistent_corpus_wide() {
    let Some(root) = omsi() else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let (mut total, mut first) = (0usize, 0usize);
    for e in std::fs::read_dir(root.join("maps"))
        .into_iter()
        .flatten()
        .flatten()
    {
        if !e.path().join("global.cfg").is_file() {
            continue;
        }
        let tiles = tiles(&e.path());
        if !tiles
            .iter()
            .any(|(_, t)| t.spline_attachments.iter().any(|a| a.repeater.is_some()))
        {
            continue;
        }
        let index = index_of(&tiles);
        for (_, t) in &tiles {
            for a in &t.spline_attachments {
                let Some(link) = a.repeater else { continue };
                total += 1;
                let Some(s) = t.splines.get(a.spline_index) else {
                    continue;
                };
                let off = index.chain_offset(s.id_code);
                let len = f64::from(s.length_m) + 0.05;
                let on = |k: u32| (-0.05..=len).contains(&(a.chainage(k) - off));
                let k = link.first_instance;
                if on(k) && (k == 0 || !on(k - 1)) {
                    first += 1;
                }
            }
        }
    }
    eprintln!("repeaters: {first} of {total} start on their own spline");
    if total > 0 {
        assert!(first as f64 >= 0.85 * total as f64, "{first} of {total}");
    }
}
