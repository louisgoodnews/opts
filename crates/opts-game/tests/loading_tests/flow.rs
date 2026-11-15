//! A clean load continues by itself and Cancel asks first.


use super::*;
use opts_game::app_state::AppState;
use opts_game::in_game::{CancelLoadingBtn, InGameEntity, InGameHudRoot};


#[test]
pub(super) fn a_clean_load_continues_by_itself_without_a_button() {
    let mut app = loading_app(
        "Mini",
        "MiniBus",
        CliArgs {
            headless: true,
            ..CliArgs::default()
        },
    );
    // Skip the loader: the world is "done" and nothing went wrong.
    {
        let mut s = app.world_mut().resource_mut::<SimulationLoadState>();
        s.stage = LoadStage::Ready;
        s.is_complete = true;
    }
    {
        let mut r = app.world_mut().resource_mut::<LoadReport>();
        r.reset();
        r.finish();
    }
    assert_eq!(outcome(&app), LoadOutcome::Ready);
    run(&mut app, 6);
    assert_eq!(screen_count(&mut app), 0, "the screen continues by itself");
    assert!(app.world().resource::<SimulationLoadState>().hud_spawned);
    let hud = app
        .world_mut()
        .query_filtered::<Entity, With<InGameHudRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(hud, 1);
}

#[test]
pub(super) fn cancel_asks_first_and_then_returns_to_plan_a_drive() {
    let root = fresh_root("cancel");
    write_map(&root, &MiniMap::simple("Mini"));
    let mut app = loading_app("Mini", "MiniBus", args_for(&root));
    run(&mut app, 2);
    let cancel = app
        .world_mut()
        .query_filtered::<Entity, With<CancelLoadingBtn>>()
        .iter(app.world())
        .next()
        .expect("cancel button");
    app.world_mut().write_message(Activated(cancel));
    run(&mut app, 3);
    assert!(
        has(&mut app, "loading.cancel-dialog"),
        "the dialog asks first"
    );
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::InGame
    );
    let keep = find_entity(app.world_mut(), "loading.cancel-keep").expect("keep");
    app.world_mut().write_message(Activated(keep));
    run(&mut app, 3);
    assert!(!has(&mut app, "loading.cancel-dialog"));
    app.world_mut().write_message(Activated(cancel));
    run(&mut app, 3);
    let yes = find_entity(app.world_mut(), "loading.cancel-yes").expect("yes");
    app.world_mut().write_message(Activated(yes));
    run(&mut app, 4);
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::VehicleSelection
    );
    let left = app
        .world_mut()
        .query_filtered::<Entity, With<InGameEntity>>()
        .iter(app.world())
        .count();
    assert_eq!(left, 0, "the session is torn down");
    let _ = std::fs::remove_dir_all(&root);
}

