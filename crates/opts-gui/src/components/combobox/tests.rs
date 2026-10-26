//! Unit tests of the combobox.

use super::build::ComboboxBuilder;
use super::search::ComboboxList;
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

#[test]
fn the_open_menu_shows_above_a_dialog_and_is_not_clipped() {
    let mut app = App::new();
    {
        let mut commands = app.world_mut().commands();
        ComboboxBuilder::new("Mirror")
            .with_options(vec!["0".into(), "1".into()])
            .spawn(&mut commands);
    }
    app.update();
    let mut menus = app
        .world_mut()
        .query_filtered::<(&GlobalZIndex, Has<bevy::ui::OverrideClip>), With<ComboboxMenu>>();
    let (z, unclipped) = menus.single(app.world()).expect("one menu");
    assert!(
        z.0 > 1000,
        "a modal is drawn at 1000: the menu must be above it"
    );
    assert!(
        unclipped,
        "the dialog's scrolling list must not cut the menu off"
    );
}

#[test]
fn opening_a_combobox_closes_the_other_one() {
    let mut app = App::new();
    app.add_message::<Activated>()
        .add_systems(Update, combobox_header_system);
    let boxes: Vec<Entity> = (0..2)
        .map(|_| {
            let mut commands = app.world_mut().commands();
            ComboboxBuilder::new("Option")
                .with_options(vec!["0".into(), "1".into()])
                .spawn(&mut commands)
        })
        .collect();
    app.update();
    let header_of = |app: &mut App, combo: Entity| {
        let mut q = app.world_mut().query::<(Entity, &ComboboxHeader)>();
        q.iter(app.world())
            .find(|(_, h)| h.0 == combo)
            .map(|(e, _)| e)
            .expect("header")
    };
    for &b in &boxes {
        let header = header_of(&mut app, b);
        app.world_mut().write_message(Activated(header));
        app.update();
    }
    let open = |app: &App, e: Entity| app.world().get::<M3Combobox>(e).unwrap().is_open;
    assert!(!open(&app, boxes[0]));
    assert!(open(&app, boxes[1]));
}
