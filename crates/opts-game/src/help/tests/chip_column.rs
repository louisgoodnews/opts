//! ISS-055: every Help key chip has one fixed width, so the descriptions share one column.

use super::*;
use crate::help::controls::{Binding, KeyChip, GROUPS, KEY_CHIP_WIDTH};

/// The description x-offset of each table row (chip width plus the row's gap). Panics on a chip
/// without a fixed width.
fn offsets(app: &mut App) -> Vec<f32> {
    let world = app.world_mut();
    let chips: Vec<(Val, Entity)> = world
        .query_filtered::<(&Node, &ChildOf), With<KeyChip>>()
        .iter(world)
        .map(|(n, p)| (n.width, p.parent()))
        .collect();
    chips
        .into_iter()
        .filter_map(|(width, row)| {
            let Val::Px(width) = width else {
                panic!("key chip without a fixed width")
            };
            let Val::Px(gap) = world.entity(row).get::<Node>()?.column_gap else {
                return None;
            };
            (world.entity(row).get::<Children>()?.len() == 2).then_some(width + gap)
        })
        .collect()
}

#[test]
fn every_key_row_has_the_same_description_offset() {
    let mut app = app();
    enter_help(&mut app);
    (0..3).for_each(|_| app.update());
    let rows = offsets(&mut app);
    let listed: usize = GROUPS.iter().map(|(_, r)| r.len()).sum();
    assert!(rows.len() >= listed, "{} rows of {listed}", rows.len());
    assert!(rows.iter().all(|o| *o == KEY_CHIP_WIDTH + 16.0), "{rows:?}");
}

#[test]
fn the_chip_width_fits_every_default_chip() {
    // Estimate at 9 px per character of the Label size, plus 8 px padding per side (not measured).
    let fits = |b: &Binding| b.shown.chars().count() as f32 * 9.0 + 16.0 <= KEY_CHIP_WIDTH;
    assert!(GROUPS.iter().flat_map(|(_, r)| r.iter()).all(fits));
}
