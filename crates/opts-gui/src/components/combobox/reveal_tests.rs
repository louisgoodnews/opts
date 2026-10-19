//! Reveal tests of the combobox.

use super::*;
use crate::testing::layout_app;

#[test]
fn opening_scrolls_the_selected_option_into_view() {
    let mut app = layout_app();
    let combo = {
        let mut commands = app.world_mut().commands();
        ComboboxBuilder::new("Start")
            .with_options((0..16).map(|i| format!("Start point {i}")).collect())
            .with_selected(14)
            .spawn(&mut commands)
    };
    for _ in 0..4 {
        app.update();
    }
    app.world_mut()
        .get_mut::<M3Combobox>(combo)
        .unwrap()
        .is_open = true;
    for _ in 0..8 {
        app.update();
    }
    let list = app
        .world_mut()
        .query_filtered::<Entity, With<ComboboxList>>()
        .single(app.world())
        .unwrap();
    let y = app.world().get::<ScrollPosition>(list).unwrap().y;
    assert!(y > 0.0, "option 14 of 16 is below the fold, scroll is {y}");
}
