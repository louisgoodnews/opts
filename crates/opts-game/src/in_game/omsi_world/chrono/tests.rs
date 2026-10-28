//! Unit tests for `super` (`chrono.rs`).

use super::*;
use opts_core::formats::chrono::ChronoEntry;
use std::path::Path;

type Ymd = (i32, u32, u32);

fn entry(
    rel: &str,
    start: Ymd,
    end: Option<Ymd>,
    tiles: &[(i32, i32)],
    marked: &[(i32, i32)],
) -> ChronoEntry {
    let d = |(y, m, day): Ymd| ChronoDate::from_ymd(y, m, day);
    ChronoEntry {
        rel: rel.into(),
        start: Some(d(start)),
        end: end.map(d),
        tiles: tiles
            .iter()
            .map(|k| (*k, PathBuf::from(format!("{rel}/tile_{}_{}.map", k.0, k.1))))
            .collect(),
        terrain_relevant: marked.iter().copied().collect(),
        ..ChronoEntry::default()
    }
}

/// A synthetic two-entry map: entry A (from 1989) edits tiles (0,0) and (5,5) and marks (0,0)
/// as terrain-relevant; entry B (1992–1993) edits (1,0).
fn two_entry_map() -> ChronoSet {
    ChronoSet::from_entries(
        Path::new("/map"),
        vec![
            entry("0200_A", (1989, 11, 10), None, &[(0, 0), (5, 5)], &[(0, 0)]),
            entry("0300_B", (1992, 1, 1), Some((1993, 12, 31)), &[(1, 0)], &[]),
        ],
    )
}

fn plan(
    set: &ChronoSet,
    from: CalendarDate,
    to: CalendarDate,
    resident: &[(i32, i32)],
    player: (i32, i32),
) -> Vec<(i32, i32)> {
    let switch = set.switch(&set.active(chrono_date(from)), &set.active(chrono_date(to)));
    rebuild_order(
        &switch.rebuild_tiles(),
        |k| resident.contains(&k),
        Some(player),
    )
}

#[test]
fn equal_active_sets_rebuild_nothing() {
    let set = two_entry_map();
    let resident: Vec<_> = (-1..=1)
        .flat_map(|x| (-1..=1).map(move |y| (x, y)))
        .collect();
    // 1986 and 1988: nothing active either time.
    assert!(set
        .switch(
            &set.active(chrono_date(CalendarDate::new(1986, 1, 1))),
            &set.active(chrono_date(CalendarDate::new(1988, 6, 1)))
        )
        .is_noop());
    assert!(plan(
        &set,
        CalendarDate::new(1990, 1, 1),
        CalendarDate::new(1991, 1, 1),
        &resident,
        (0, 0)
    )
    .is_empty());
}

/// 1986 → 1990 toggles A: its tiles plus the 3 × 3 ring of its marked tile; only resident tiles;
/// the player's tile first.
#[test]
fn switch_rebuilds_changed_and_ring_tiles_that_are_resident_player_first() {
    let set = two_entry_map();
    let resident: Vec<_> = (0..=2).flat_map(|x| (0..=2).map(move |y| (x, y))).collect();
    let order = plan(
        &set,
        CalendarDate::new(1986, 1, 1),
        CalendarDate::new(1990, 1, 1),
        &resident,
        (1, 1),
    );
    assert_eq!(order.first(), Some(&(1, 1)), "{order:?}");
    // The ring around (0,0) that is resident: (0,0),(0,1),(1,0),(1,1); (5,5) is not resident.
    let mut sorted = order.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, vec![(0, 0), (0, 1), (1, 0), (1, 1)]);
    // 1990 → 1992 toggles B only: tile (1,0), no ring.
    let order = plan(
        &set,
        CalendarDate::new(1990, 1, 1),
        CalendarDate::new(1992, 6, 1),
        &resident,
        (2, 2),
    );
    assert_eq!(order, vec![(1, 0)]);
    // The player's tile leads even when other tiles have smaller keys.
    let order = plan(
        &set,
        CalendarDate::new(1986, 1, 1),
        CalendarDate::new(1992, 6, 1),
        &resident,
        (1, 0),
    );
    assert_eq!(order[0], (1, 0));
    assert_eq!(order.len(), 4);
}
