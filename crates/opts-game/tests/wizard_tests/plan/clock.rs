use super::*;

/// Writes `maps/<map>/laststn.osn` with a `[time]` block into the pinned fixture root.
fn write_osn(map: &str, hour: u32, minute: u32) {
    let root = std::env::temp_dir().join(format!("opts-wizard-root-{}", std::process::id()));
    let dir = root.join("maps").join(map);
    std::fs::create_dir_all(&dir).expect("map dir");
    let text = format!(
        "[name]\r\nLetzte Situation\r\n\r\n[time]\r\n1992\r\n76\r\n{hour}\r\n{minute}\r\n0.35\r\n\r\n[egopos]\r\n0\r\n"
    );
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    std::fs::write(dir.join("laststn.osn"), bytes).expect("write osn");
}

fn choose_in_where(app: &mut App, id: &str) {
    use opts_game::new_game_screen::step_where::StepList;
    let root = app
        .world_mut()
        .query::<(Entity, &StepList)>()
        .iter(app.world())
        .find(|(_, s)| s.0 == PlannerStep::Where)
        .map(|(e, _)| e)
        .expect("the Where list");
    app.world_mut()
        .write_message(opts_gui::patterns::master_detail::MasterDetailSelected {
            root,
            id: Some(id.to_owned()),
        });
    for _ in 0..4 {
        app.update();
    }
}

/// The source label of the time that is shown, with its text (a `Display::None` one is not).
fn shown_time_source(
    app: &mut App,
) -> Option<(opts_game::new_game_screen::planner::TimeSource, String)> {
    use opts_game::new_game_screen::step_when::TimeSourceLabel;
    let mut q = app.world_mut().query::<(&TimeSourceLabel, &Node, &Text)>();
    let shown: Vec<_> = q
        .iter(app.world())
        .filter(|(_, n, _)| n.display != Display::None)
        .map(|(l, _, t)| (l.0, t.0.clone()))
        .collect();
    assert!(shown.len() <= 1, "two source labels at once: {shown:?}");
    shown.into_iter().next()
}

fn clock(app: &App) -> (u32, u32) {
    let s = app.world().resource::<NewGameSetup>();
    (s.hour, s.minute)
}

fn wall_minutes() -> i64 {
    use chrono::Timelike;
    let t = chrono::Local::now().time();
    i64::from(t.hour() * 60 + t.minute())
}

/// Minutes between two times of day, around midnight.
fn apart(a: i64, b: i64) -> i64 {
    let d = (a - b).rem_euclid(1440);
    d.min(1440 - d)
}

#[test]
fn a_map_without_a_saved_clock_starts_at_the_current_time_and_says_so() {
    let before = wall_minutes();
    let mut app = wizard("clock-now");
    let after = wall_minutes();
    let (h, m) = clock(&app);
    let shown = i64::from(h * 60 + m);
    assert!(
        apart(shown, before) <= 1 || apart(shown, after) <= 1,
        "{h:02}:{m:02} is not the wall clock ({before}..{after})"
    );
    let state = app.world().resource::<PlannerState>().clone();
    assert_eq!(
        state.time_source,
        Some(opts_game::new_game_screen::planner::TimeSource::Now)
    );
    assert!(!state.time_chosen);
    press(&mut app, "wizard.stepper.when");
    assert_eq!(
        shown_time_source(&mut app),
        Some((
            opts_game::new_game_screen::planner::TimeSource::Now,
            "Now".into()
        ))
    );
    cleanup("clock-now");
}

#[test]
fn a_map_with_a_saved_clock_starts_at_that_time_and_says_so() {
    write_osn("Generic_Map_07", 21, 33);
    let mut app = wizard("clock-map");
    choose_in_where(&mut app, "Generic_Map_07");
    assert_eq!(clock(&app), (21, 33));
    let state = app.world().resource::<PlannerState>().clone();
    assert_eq!(
        state.time_source,
        Some(opts_game::new_game_screen::planner::TimeSource::Map)
    );
    press(&mut app, "wizard.stepper.when");
    assert_eq!(
        shown_time_source(&mut app),
        Some((
            opts_game::new_game_screen::planner::TimeSource::Map,
            "Map default".into()
        ))
    );
    // Editing the time makes it the user's: the label goes away.
    press(&mut app, "wizard.when.hour.plus");
    assert_eq!(clock(&app), (22, 33));
    assert!(app.world().resource::<PlannerState>().time_chosen);
    assert_eq!(shown_time_source(&mut app), None);
    cleanup("clock-map");
}

#[test]
fn changing_the_map_updates_the_time_until_the_user_has_edited_it() {
    write_osn("Generic_Map_11", 6, 15);
    let mut app = wizard("clock-switch");
    choose_in_where(&mut app, "Generic_Map_11");
    assert_eq!(clock(&app), (6, 15));
    // Another map without a clock: back to the wall clock.
    let before = wall_minutes();
    choose_in_where(&mut app, "Generic_Map_09");
    let (h, m) = clock(&app);
    assert!(apart(i64::from(h * 60 + m), before) <= 2, "{h:02}:{m:02}");
    choose_in_where(&mut app, "Generic_Map_11");
    assert_eq!(clock(&app), (6, 15));
    // After an edit the map switch keeps the user's time.
    press(&mut app, "wizard.stepper.when");
    press(&mut app, "wizard.when.quick.3");
    assert_eq!(clock(&app), (18, 0));
    press(&mut app, "wizard.stepper.where");
    choose_in_where(&mut app, "Generic_Map_09");
    assert_eq!(clock(&app), (18, 0), "the user's time stays");
    cleanup("clock-switch");
}
