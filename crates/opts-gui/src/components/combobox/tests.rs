//! Unit tests of the combobox.

use super::*;

#[test]
fn test_combobox_builder_spawn() {
    let mut app = App::new();
    let entity = {
        let mut commands = app.world_mut().commands();
        ComboboxBuilder::new("Route")
            .with_options(vec!["Line 100".into(), "Line 200".into()])
            .with_selected(1)
            .spawn(&mut commands)
    };

    app.update();

    let combo = app.world().get::<M3Combobox>(entity);
    assert_eq!(combo.map(|c| c.selected), Some(Some(1)));
    assert_eq!(combo.map(|c| c.is_open), Some(false));
}

#[test]
fn a_long_menu_scrolls_instead_of_clipping() {
    let mut app = App::new();
    let options: Vec<String> = (0..16).map(|i| format!("Start point {i}")).collect();
    {
        let mut commands = app.world_mut().commands();
        ComboboxBuilder::new("Start")
            .with_options(options)
            .spawn(&mut commands);
    }
    app.update();

    let mut lists = app
        .world_mut()
        .query_filtered::<(&Node, &ScrollPosition), With<ComboboxList>>();
    let (node, _) = lists.single(app.world()).expect("one list");
    assert_eq!(
        node.overflow.y,
        OverflowAxis::Scroll,
        "a list taller than the menu must scroll (the 16 start points of Berlin-Spandau)"
    );
}

#[test]
fn folding_ignores_case_and_diacritics() {
    assert_eq!(fold("Süd-Straße Café Œuvre"), "sud-strasse cafe oeuvre");
    assert!(matches_query("Berlin Südkreuz", "sud"));
    assert!(matches_query("Berlin Südkreuz", "SÜD kreuz"));
    assert!(matches_query("Kreuzberg", "kreuz berg"));
    assert!(!matches_query("Berlin Südkreuz", "nord"));
    assert!(matches_query("U-Bhf. Ruhleben", "ubhf"));
    assert!(matches_query("U-Bhf. Ruhleben", "bhf ruhl"));
    assert!(matches_query("anything", "   "));
}
