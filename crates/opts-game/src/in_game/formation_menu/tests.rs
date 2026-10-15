//! Tests of the editor's state and actions (no installation needed; spawning is checked live).

use super::state::{Act, Car, FormationCommands, FormationEditor, Summary};
use super::*;
use crate::in_game::vehicle_menu::VehicleMenu;
use crate::session_vehicles::SessionVehicles;
use opts_core::formats::trains::Orientation;
use opts_core::formats::trains::{FormationIssue, FormationSource, TrainFormation};
use opts_gui::interaction::events::ButtonClicked;

fn app() -> App {
    let mut app = App::new();
    app.add_message::<ButtonClicked>()
        .init_resource::<FormationEditor>()
        .init_resource::<FormationCommands>()
        .init_resource::<VehicleMenu>()
        .init_resource::<SessionVehicles>()
        .add_systems(Update, actions::formation_actions);
    {
        let mut ed = app.world_mut().resource_mut::<FormationEditor>();
        for (rel, label) in [
            ("Vehicles/BR218/BR218_OR.ovh", "BR218 / BR218_OR  (80 t)"),
            ("Vehicles/Bm238/By438_mi.ovh", "Bm238 / By438_mi  (40 t)"),
            ("Vehicles/Gueterwagen/Zas.ovh", "Gueterwagen / Zas  (40 t)"),
        ] {
            ed.cars.push(Car {
                rel: rel.into(),
                label: label.into(),
                mass_t: Some(40.0),
            });
        }
        ed.presets = vec![
            "Trains/regio_grundorf.zug".into(),
            "Trains/X10_ICE1.zug".into(),
        ];
    }
    app
}

fn say(app: &mut App, lines: &[&str]) {
    app.world_mut()
        .resource_mut::<FormationCommands>()
        .0
        .extend(lines.iter().map(|l| (*l).to_owned()));
    app.update();
}

fn ed(app: &App) -> &FormationEditor {
    app.world().resource::<FormationEditor>()
}

fn files(app: &App) -> Vec<String> {
    ed(app)
        .formation()
        .segments
        .iter()
        .map(|s| {
            s.vehicle
                .as_str()
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .collect()
}

#[test]
fn adding_cars_by_name_builds_the_train_in_order() {
    let mut app = app();
    say(&mut app, &["add br218", "add by438", "add zas"]);
    assert_eq!(files(&app), ["BR218_OR.ovh", "By438_mi.ovh", "Zas.ovh"]);
    assert_eq!(ed(&app).selected, Some(2), "the new car is selected");
    say(&mut app, &["add no such car"]);
    assert_eq!(ed(&app).formation().len(), 3);
    assert!(
        ed(&app).status.contains("No rail car"),
        "{}",
        ed(&app).status
    );
}

#[test]
fn the_edit_commands_count_cars_from_one_and_ignore_nonsense() {
    let mut app = app();
    say(&mut app, &["add br218", "add by438", "add zas"]);
    say(&mut app, &["up 3"]);
    assert_eq!(files(&app), ["BR218_OR.ovh", "Zas.ovh", "By438_mi.ovh"]);
    say(&mut app, &["down 1"]);
    assert_eq!(files(&app), ["Zas.ovh", "BR218_OR.ovh", "By438_mi.ovh"]);
    say(&mut app, &["up 1", "down 3", "up 0", "turn 9", "remove x"]);
    assert_eq!(files(&app), ["Zas.ovh", "BR218_OR.ovh", "By438_mi.ovh"]);
    say(&mut app, &["turn 2"]);
    assert_eq!(
        ed(&app).formation().segments[1].orientation,
        Orientation::Backward
    );
    say(&mut app, &["power 3"]);
    assert_eq!(ed(&app).formation().segments[2].powered, Some(true));
    say(&mut app, &["remove 1"]);
    assert_eq!(files(&app), ["BR218_OR.ovh", "By438_mi.ovh"]);
    assert_eq!(ed(&app).selected, None);
}

#[test]
fn reverse_turns_the_whole_train_and_clear_empties_it() {
    let mut app = app();
    say(&mut app, &["add br218", "add by438", "turn 2"]);
    say(&mut app, &["reverse"]);
    assert_eq!(files(&app), ["By438_mi.ovh", "BR218_OR.ovh"]);
    let o: Vec<_> = ed(&app)
        .formation()
        .segments
        .iter()
        .map(|s| s.orientation)
        .collect();
    assert_eq!(o, [Orientation::Forward, Orientation::Backward]);
    say(&mut app, &["clear"]);
    assert!(ed(&app).formation().is_empty());
    assert_eq!(ed(&app).formation().name, "Formation");
}

#[test]
fn the_search_filters_both_lists_by_every_word() {
    let mut app = app();
    say(&mut app, &["search bm238 438"]);
    let shown = ed(&app).shown_list();
    assert_eq!(shown.len(), 1);
    assert!(shown[0].1.contains("By438"));
    say(&mut app, &["search"]);
    assert_eq!(ed(&app).shown_list().len(), 3);
    say(&mut app, &["trains", "search ice"]);
    let shown = ed(&app).shown_list();
    assert_eq!(
        shown,
        vec![(1, "X10_ICE1.zug".to_owned())],
        "the folder is trimmed"
    );
}

#[test]
fn spawn_needs_two_cars_a_clean_check_and_an_entry_point() {
    let mut app = app();
    say(&mut app, &["add br218"]);
    let e = ed(&app);
    assert!(!e.can_spawn(Some(1)), "one car");
    let mut e = FormationEditor::default();
    e.formation.add_file("a.ovh");
    e.formation.add_file("b.ovh");
    e.validated = e.revision;
    assert!(e.can_spawn(Some(1)));
    assert!(!e.can_spawn(None), "no entry point");
    e.issues.push(FormationIssue::NoCab);
    assert!(!e.can_spawn(Some(1)), "an open problem");
    e.issues.clear();
    e.touch();
    assert!(!e.can_spawn(Some(1)), "not checked since the last change");
}

#[test]
fn rows_and_summary_describe_the_train() {
    let mut e = FormationEditor::default();
    e.formation.add_file("Vehicles/A/loco.ovh");
    e.formation.add_file("Vehicles/A/wagon.ovh");
    e.formation.turn_car(1);
    e.formation.cycle_powered(1);
    e.formation.cycle_powered(1);
    e.summary = Summary {
        parts: 2,
        mass_t: 120.0,
    };
    assert_eq!(e.row_texts(), ["1  loco.ovh", "2  wagon.ovh turned hauled"]);
    assert_eq!(e.summary_text(), "2 cars, 2 vehicle files, about 120 t");
}

#[test]
fn a_clicked_button_runs_its_action() {
    let mut app = app();
    let pick = app.world_mut().spawn(Act::Pick(1)).id();
    let add = app.world_mut().spawn(Act::Add).id();
    app.world_mut().write_message(ButtonClicked(pick));
    app.world_mut().write_message(ButtonClicked(add));
    app.update();
    assert_eq!(files(&app), ["By438_mi.ovh"]);
    let up = app.world_mut().spawn(Act::Add).id();
    // Without a pick Add is refused with a hint, not a panic: clear the pick via the list change.
    app.world_mut().resource_mut::<FormationEditor>().pick = None;
    app.world_mut().write_message(ButtonClicked(up));
    app.update();
    assert_eq!(ed(&app).formation().len(), 1);
    assert!(ed(&app).status.contains("Pick a rail car"));
}

#[test]
fn every_change_asks_for_a_rebuild_and_a_new_check() {
    let mut app = app();
    let before = ed(&app).revision;
    say(&mut app, &["add br218"]);
    assert!(ed(&app).revision > before);
    assert_ne!(ed(&app).validated, ed(&app).revision);
}

#[test]
fn a_loaded_formation_replaces_the_train_and_keeps_its_source() {
    let mut app = app();
    say(&mut app, &["add br218"]);
    let (loaded, _) = TrainFormation::from_zug(b"a.ovh\r\n0\r\nb.ovh\r\n1", "z", "Trains\\z.zug")
        .expect("parses");
    app.world_mut()
        .resource_mut::<FormationEditor>()
        .set_formation(loaded);
    assert_eq!(ed(&app).formation().len(), 2);
    assert_eq!(
        ed(&app).formation().source,
        FormationSource::ZugFile("Trains\\z.zug".into())
    );
    assert_eq!(ed(&app).selected, None);
}
