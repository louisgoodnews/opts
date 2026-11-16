//! Walking the steps, choosing and driving.

use super::*;

#[test]
fn the_wizard_opens_on_where_with_the_map_list_and_the_summary() {
    let mut app = wizard("open");
    assert_eq!(step(&app), PlannerStep::Where);
    let t = texts(&mut app);
    assert!(t.iter().any(|x| x == "Plan a drive"), "{t:?}");
    assert!(
        t.iter().any(|x| x.starts_with("Berlin-Spandau")),
        "summary names the map"
    );
    assert!(
        t.iter().any(|x| x.starts_with("Search 42 maps")),
        "the search field counts the maps: {:?}",
        t.iter()
            .filter(|x| x.contains("Search"))
            .collect::<Vec<_>>()
    );
    cleanup("open");
}

#[test]
fn next_back_skip_and_escape_walk_the_steps() {
    let mut app = wizard("walk");
    press(&mut app, "wizard.footer.next");
    assert_eq!(step(&app), PlannerStep::What);
    press(&mut app, "wizard.footer.next");
    assert_eq!(step(&app), PlannerStep::When);
    press(&mut app, "wizard.footer.back");
    assert_eq!(step(&app), PlannerStep::What);
    press(&mut app, "wizard.footer.skip");
    assert_eq!(step(&app), PlannerStep::Review);
    act(&mut app, UiAction::Back);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(step(&app), PlannerStep::When, "Esc goes one step back");
    act(&mut app, UiAction::Back);
    act(&mut app, UiAction::Back);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(step(&app), PlannerStep::Where);
    act(&mut app, UiAction::Back);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(state(&app), AppState::MainMenu, "then out of the wizard");
    cleanup("walk");
}

#[test]
fn drive_starts_the_session_and_remembers_the_choices() {
    let mut app = wizard("drive");
    press(&mut app, "wizard.footer.skip");
    assert_eq!(step(&app), PlannerStep::Review);
    press(&mut app, "wizard.footer.drive");
    assert_eq!(state(&app), AppState::InGame);
    let saved = DriveHistory::load_from(&history_path("drive"));
    let setup = app.world().resource::<NewGameSetup>().clone();
    let last = saved.last().expect("the drive is in the history file");
    assert_eq!(last.map, setup.selected_map);
    assert_eq!(last.vehicle, setup.selected_vehicle);
    assert_eq!((last.hour, last.minute), (setup.hour, setup.minute));
    assert!(last.last_driven > 0);
    cleanup("drive");
}

pub(super) fn choose_in_what(app: &mut App, id: &str) {
    use opts_game::new_game_screen::step_where::StepList;
    let root = app
        .world_mut()
        .query::<(Entity, &StepList)>()
        .iter(app.world())
        .find(|(_, s)| s.0 == PlannerStep::What)
        .map(|(e, _)| e)
        .expect("the What list");
    app.world_mut()
        .write_message(opts_gui::patterns::master_detail::MasterDetailSelected {
            root,
            id: Some(id.to_owned()),
        });
    for _ in 0..4 {
        app.update();
    }
}

mod validation;
mod vehicle;
