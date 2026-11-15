//! The bent crossings: they lie on or above the terrain and their arms meet the spline ends.

use super::*;

#[test]
fn spandau_bent_crossings_lie_on_the_terrain_and_meet_their_arms() {
    let Some(mut map) = Map::open("Berlin-Spandau") else {
        eprintln!("OMSI_PATH not set or Berlin-Spandau not installed; skipping");
        return;
    };
    let keys: Vec<(i32, i32)> = map.files.keys().copied().collect();
    let mut terrain = HashMap::new();
    let mut tally = Tally::default();
    for key in keys {
        let Some(tile) = map.tile(key) else { continue };
        for o in &tile.objects {
            let Some((path, sco)) = map.sco(&o.sco_file) else {
                continue;
            };
            let Some((meshes, base)) = bent_meshes(&map, &path, &sco) else {
                continue;
            };
            tally.placements += 1;
            tally.add_ground(&map, key, o, (&meshes, &base), &mut terrain);
            tally.add_arms(&mut map, key, o, &sco.bent(&base));
        }
    }
    let Tally {
        placements,
        area,
        below,
        arms,
        arm_misses,
    } = tally;
    let share = 100.0 * below / area.max(1.0);
    eprintln!(
        "bent placements {placements}, ground {area:.0} m2, below terrain {share:.3} %, arms {arms}, height misses {}",
        arm_misses.len()
    );
    assert_eq!(placements, 11);
    assert!(
        share < 0.1,
        "{share:.3} % of the bent crossing ground lies below the terrain"
    );
    assert!(arms > 0);
    // 38 of 39 arms meet their spline within 1 cm; one (`Einm_SpandauerD_WiesenD`) is 1.3 cm
    // off. None may be further than 2 cm.
    assert!(
        arm_misses.len() <= 1,
        "arms off their spline: {arm_misses:?}"
    );
    assert!(
        arm_misses.iter().all(|(_, _, dz)| dz.abs() < 0.02),
        "arms off their spline: {arm_misses:?}"
    );
}
