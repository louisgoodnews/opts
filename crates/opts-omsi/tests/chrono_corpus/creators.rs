//! Creator tables of the Chrono corpus test: which entry adds an element that a later overlay
//! selects, and whether that creator is applied before the selection.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use opts_omsi::formats::chrono::{ChronoSet, OverlayOp, TileOverlay};
use opts_omsi::formats::map::MapTile;

pub(super) fn build_chrono_creators(
    set: &ChronoSet,
    overlay_cache: &mut HashMap<PathBuf, TileOverlay>,
) -> BTreeMap<((i32, i32), u64), Vec<usize>> {
    let mut creators: BTreeMap<((i32, i32), u64), Vec<usize>> = BTreeMap::new();
    for (i, e) in set.entries().iter().enumerate() {
        for (key, path) in &e.tiles {
            let ov = overlay_cache
                .entry(path.clone())
                .or_insert_with(|| TileOverlay::parse(&std::fs::read(path).unwrap_or_default()));
            for op in &ov.ops {
                if let OverlayOp::Add(t) = op {
                    let ids = t
                        .objects
                        .iter()
                        .map(|o| o.id_code)
                        .chain(t.splines.iter().map(|s| s.id_code))
                        .chain(t.attached_objects.iter().map(|a| a.id_code))
                        .chain(t.spline_attachments.iter().map(|a| a.id_code));
                    for id in ids {
                        creators.entry((*key, id)).or_default().push(i);
                    }
                }
            }
        }
    }
    creators
}

pub(super) fn check_selection_creator_order(
    base: &MapTile,
    ov: &TileOverlay,
    key: (i32, i32),
    i: usize,
    creators: &BTreeMap<((i32, i32), u64), Vec<usize>>,
    created_earlier: &mut usize,
    created_later: &mut usize,
) {
    for op in &ov.ops {
        let id = match op {
            OverlayOp::SelectObject(id) | OverlayOp::SelectSpline(id) => *id,
            _ => continue,
        };
        let in_base = base.objects.iter().any(|o| o.id_code == id)
            || base.splines.iter().any(|s| s.id_code == id)
            || base.attached_objects.iter().any(|a| a.id_code == id)
            || base.spline_attachments.iter().any(|a| a.id_code == id);
        if in_base {
            continue;
        }
        if let Some(c) = creators.get(&(key, id)) {
            if c.iter().any(|&c| c < i) {
                *created_earlier += 1;
            } else if !c.contains(&i) {
                *created_later += 1;
            }
        }
    }
}
