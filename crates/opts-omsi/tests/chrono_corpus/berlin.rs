//! Berlin-Spandau, 1986 against 1994: the overlays of the entries active in each year differ.

use std::path::PathBuf;

use opts_omsi::formats::chrono::{ChronoDate, ChronoSet, TileOverlay};
use opts_omsi::formats::map::MapTile;

/// Berlin-Spandau, 1986 against 1994: the overlays of the fifteen entries active in 1994 change
/// the tiles and the selections on them resolve.
#[test]
fn berlin_spandau_1986_and_1994_differ() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let map = root.join("maps/Berlin-Spandau");
    if !map.is_dir() {
        eprintln!("Berlin-Spandau not installed; skipping");
        return;
    }
    let set = ChronoSet::scan(&map);
    let (a, b) = (
        set.active(ChronoDate::from_ymd(1986, 6, 1)),
        set.active(ChronoDate::from_ymd(1994, 12, 31)),
    );
    assert_eq!(a.indices().len(), 1, "only the 1986 fare entry");
    assert_eq!(b.indices().len(), 15);
    let switch = set.switch(&a, &b);
    eprintln!(
        "Spandau 1986 -> 1994: {} entries toggled, {} tiles changed, ring {}, data files {:?}",
        switch.toggled.len(),
        switch.tiles.len(),
        switch.carve_ring.len(),
        switch.data_files
    );
    assert!(switch.touches("ailists_#upd.cfg") && switch.touches_prefix("parklist_p"));
    let key = (2393, 11285);
    assert!(switch.tiles.contains(&key));
    let base = MapTile::parse(&std::fs::read(map.join("tile_2393_11285.map")).unwrap()).unwrap();
    let mut tile = base.clone();
    let (mut sel, mut res) = (0, 0);
    for p in set.plan(&b).overlays(key) {
        let r = tile.apply_overlay(&TileOverlay::parse(&std::fs::read(p).unwrap()));
        sel += r.selections;
        res += r.resolved;
    }
    eprintln!(
        "tile_2393_11285: objects {} -> {}, splines {} -> {}, attached {} -> {}; {res} of {sel} selections",
        base.objects.len(),
        tile.objects.len(),
        base.splines.len(),
        tile.splines.len(),
        base.attached_objects.len(),
        tile.attached_objects.len()
    );
    assert_ne!(tile, base);
    // The one miss is a double delete: `0205_OeffnungFalkenseer` and `0500_Mauerabriss` both
    // delete the repeater 127809.
    assert_eq!(res + 1, sel);
}
