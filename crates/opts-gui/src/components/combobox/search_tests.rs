//! Search tests of the combobox.

use super::build::ComboboxBuilder;
use super::search::{ComboboxList, ComboboxNoMatch, ComboboxSearch};
use super::*;
use crate::actions::{Direction, UiAction};
use crate::components::text_field::TextFieldClear;
use crate::testing::{act, focused, headless_app, wheel};
use std::time::Duration;

const STEP_MS: u64 = 50;

fn app_with(options: usize, selected: Option<usize>) -> (App, Entity) {
    app_with_id(options, selected, None)
}

fn app_with_id(options: usize, selected: Option<usize>, id: Option<&'static str>) -> (App, Entity) {
    let mut app = headless_app();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(STEP_MS),
    ));
    let opts: Vec<String> = (0..options).map(|i| format!("Start point {i}")).collect();
    let combo = {
        let mut commands = app.world_mut().commands();
        let mut b = ComboboxBuilder::new("Start").with_options(opts);
        if let Some(i) = selected {
            b = b.with_selected(i);
        }
        let e = b.spawn(&mut commands);
        if let Some(id) = id {
            commands.entity(e).insert(crate::ui_id::UiId::new(id));
        }
        e
    };
    for _ in 0..3 {
        app.update();
    }
    (app, combo)
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

fn open(app: &mut App, combo: Entity) {
    app.world_mut()
        .get_mut::<M3Combobox>(combo)
        .unwrap()
        .is_open = true;
    frames(app, 4);
}

fn search_field(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<ComboboxSearch>>()
        .single(app.world())
        .expect("a search field")
}

fn type_into(app: &mut App, text: &str) {
    let field = search_field(app);
    app.world_mut().get_mut::<M3TextField>(field).unwrap().text = text.to_string();
    frames(app, 2);
}

/// Indices of the options that are shown.
fn shown(app: &mut App) -> Vec<usize> {
    let mut q = app.world_mut().query::<(&ComboboxMenuItem, &Node)>();
    let mut v: Vec<usize> = q
        .iter(app.world())
        .filter(|(_, n)| n.display != Display::None)
        .map(|(i, _)| i.index)
        .collect();
    v.sort_unstable();
    v
}

fn no_match_shown(app: &mut App) -> bool {
    let mut q = app.world_mut().query::<(&ComboboxNoMatch, &Node)>();
    q.iter(app.world()).any(|(_, n)| n.display != Display::None)
}

fn state(app: &App, combo: Entity) -> (bool, Option<usize>) {
    let c = app.world().get::<M3Combobox>(combo).unwrap();
    (c.is_open, c.selected)
}

mod filter_tests;
mod keyboard_tests;
mod labels_tests;
