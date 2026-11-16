//! Only minor problems continue by themselves, and are announced once.

use super::*;

#[test]
fn only_minor_problems_continue_by_themselves_and_are_announced_once() {
    use opts_game::in_game::load_report::LoadProblem;
    let mut app = loading_app(
        "Mini",
        "MiniBus",
        CliArgs {
            headless: true,
            ..CliArgs::default()
        },
    );
    {
        let mut s = app.world_mut().resource_mut::<SimulationLoadState>();
        s.stage = LoadStage::Ready;
        s.is_complete = true;
        let mut r = app.world_mut().resource_mut::<LoadReport>();
        r.reset();
        r.add(LoadProblem::counted(
            "load-problem-files",
            2,
            0,
            "N2_White.tga",
        ));
        r.finish();
    }
    assert_eq!(
        outcome(&app),
        LoadOutcome::Ready,
        "minor problems do not block"
    );
    run(&mut app, 6);
    assert_eq!(screen_count(&mut app), 0, "the screen continues by itself");
    run(&mut app, 4);
    let t = texts(&mut app);
    assert_eq!(
        t.iter().filter(|x| x.starts_with("Loaded with")).count(),
        1,
        "one toast: {t:?}"
    );
    assert!(
        t.iter().any(|x| x == "Loaded with 1 minor problem"),
        "{t:?}"
    );
    assert_eq!(
        app.world().resource::<LoadReport>().minor_count(),
        1,
        "kept in the report"
    );
    // Details opens the list.
    let action = app
        .world_mut()
        .query::<(Entity, &opts_gui::components::toast::ToastActionButton)>()
        .iter(app.world())
        .next()
        .map(|(e, _)| e)
        .expect("the toast has a Details action");
    app.world_mut().write_message(Activated(action));
    run(&mut app, 3);
    assert!(has(&mut app, "loading.minor-dialog"));
    let t = texts(&mut app);
    assert!(
        t.iter().any(|x| x.contains("files could not be read")),
        "{t:?}"
    );
    run(&mut app, 30);
    let n = texts(&mut app)
        .iter()
        .filter(|x| x.starts_with("Loaded with"))
        .count();
    assert!(n <= 1, "announced once");
}
