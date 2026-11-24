//! The chips of the Help controls table against the registry (M-0164, ISS-015): a rebindable row
//! prints the default chord of each of its actions and nothing else, and lists the chord's key.

use std::collections::HashMap;

use bevy::prelude::KeyCode;
use bevy::prelude::*;
use opts_bevy::action_keys::{action, display_on};
use opts_bevy::KeyChord;
use opts_game::help::controls::{spawn_controls, Binding, KeyChip, GROUPS, KEY_CHIP_WIDTH};

/// The letters of the keyboard the rows are printed for: a German layout, where the key at the Z
/// position says Y (the rail lever row prints "Q / Y").
fn german_letters() -> HashMap<KeyCode, char> {
    [(KeyCode::KeyZ, 'Y'), (KeyCode::KeyY, 'Z')]
        .into_iter()
        .collect()
}

/// The texts a chip may print for `chord`: the registry's print ([`display_on`]) and, for the key
/// at the Equal position, the "+" of the zoom row (Shift+= on a US keyboard).
fn accepted_prints(chord: KeyChord) -> Vec<String> {
    let mut out: Vec<String> = display_on(chord, &german_letters()).into_iter().collect();
    if chord.key == KeyCode::Equal && !chord.shift && !chord.ctrl {
        out.push("+".to_owned());
    }
    out
}

#[test]
fn every_chip_of_a_rebindable_row_names_the_chord_of_its_action() {
    for (_, rows) in GROUPS {
        for b in rows.iter().filter(|b| !b.ids.is_empty()) {
            let tokens: Vec<&str> = b
                .shown
                .split(|c: char| c == '/' || c.is_whitespace())
                .filter(|t| !t.is_empty())
                .collect();
            let mut printed_by_action = Vec::new();
            for id in b.ids {
                let def = action(id)
                    .unwrap_or_else(|| panic!("{}: no action {id} in the registry", b.shown));
                let chord = KeyChord::parse(def.default)
                    .unwrap_or_else(|e| panic!("{id}: default {} is not a chord: {e}", def.default))
                    .unwrap_or_else(|| panic!("{id}: default is none"));
                let prints = accepted_prints(chord);
                assert!(
                    tokens.iter().any(|t| prints.iter().any(|p| p == t)),
                    "the chip \"{}\" does not print {id} (default {}, printed {prints:?})",
                    b.shown,
                    def.default
                );
                assert!(
                    b.keys.contains(&chord.key),
                    "the row \"{}\" does not list {:?}, the key of {id}",
                    b.shown,
                    chord.key
                );
                printed_by_action.push(prints);
            }
            for t in &tokens {
                assert!(
                    printed_by_action
                        .iter()
                        .any(|prints| prints.iter().any(|p| p == t)),
                    "the chip \"{}\" prints {t}, which none of its actions {:?} is bound to",
                    b.shown,
                    b.ids
                );
            }
        }
    }
}

/// One laid-out table row: the key chip, its text and the description text, as rects `[x, y, w, h]`.
struct LaidRow {
    chip: [f32; 4],
    chip_text: [f32; 4],
    desc: [f32; 4],
}

/// Spawns the controls table in a column `width` px wide with Bevy's real layout (no window) and
/// returns every key row's rects. The widths are the Help content column at the two sizes.
fn laid_out_table(width: f32) -> Vec<LaidRow> {
    use opts_gui::ui_tree::rect_of;
    let mut app = opts_gui::testing::layout_app();
    let root = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(width),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    {
        let mut commands = app.world_mut().commands();
        commands.entity(root).with_children(spawn_controls);
    }
    app.world_mut().flush();
    for _ in 0..4 {
        app.update();
    }
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<KeyChip>>()
        .iter(world)
        .map(|chip| {
            let row = world.get::<ChildOf>(chip).unwrap().parent();
            let row_children = world.get::<Children>(row).unwrap();
            let chip_text = world.get::<Children>(chip).unwrap()[0];
            LaidRow {
                chip: rect_of(world, chip).expect("chip laid out"),
                chip_text: rect_of(world, chip_text).expect("chip text laid out"),
                desc: rect_of(world, row_children[1]).expect("description laid out"),
            }
        })
        .collect()
}

#[test]
fn laid_out_descriptions_share_one_x_at_both_sizes() {
    // 1920 wide (Help content), 1024 wide (the content column at the minimum window size) and a
    // narrower column, where the rows no longer fit and flex shrink would narrow the chips.
    for width in [1300.0_f32, 500.0, 300.0] {
        let rows = laid_out_table(width);
        let listed: usize = GROUPS.iter().map(|(_, r)| r.len()).sum();
        assert!(rows.len() >= listed, "{} rows of {listed}", rows.len());
        let x0 = rows[0].desc[0];
        for (i, r) in rows.iter().enumerate() {
            assert!(
                (r.chip[2] - KEY_CHIP_WIDTH).abs() < 0.5,
                "width {width}: row {i} chip is {} px, not {KEY_CHIP_WIDTH}",
                r.chip[2]
            );
            assert!(
                (r.desc[0] - x0).abs() < 0.5,
                "width {width}: row {i} description starts at {}, not {x0}",
                r.desc[0]
            );
        }
    }
}

#[test]
fn laid_out_chip_text_fits_one_line_inside_its_chip() {
    for width in [1300.0_f32, 500.0, 300.0] {
        let rows = laid_out_table(width);
        let height = rows[0].chip[3];
        for (i, r) in rows.iter().enumerate() {
            assert!(
                (r.chip[3] - height).abs() < 0.5,
                "width {width}: row {i} chip is {} px tall, not {height} (text wrapped)",
                r.chip[3]
            );
            let right = r.chip[0] + r.chip[2];
            assert!(
                r.chip_text[0] >= r.chip[0] && r.chip_text[0] + r.chip_text[2] <= right,
                "width {width}: row {i} chip text {:?} is outside its chip {:?}",
                r.chip_text,
                r.chip
            );
        }
    }
}

#[test]
fn the_chip_width_fits_every_default_chip() {
    // Estimate at 9 px per character of the Label size, plus 8 px padding per side (not measured).
    let fits = |b: &Binding| b.shown.chars().count() as f32 * 9.0 + 16.0 <= KEY_CHIP_WIDTH;
    assert!(GROUPS.iter().flat_map(|(_, r)| r.iter()).all(fits));
}
