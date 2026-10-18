//! Tests of the *Plan a drive* wizard in Bevy's real UI layout, on the fixture catalogue.

use bevy::prelude::*;
use opts_game::{
    app_state::AppState,
    drive_history::{DriveHistory, DriveRecord},
    new_game_screen::{
        FooterButton, NewGameMenuPlugin, NewGameSetup, PlannerHistory, PlannerState, PlannerStep,
    },
    settings::GameSettings,
};
use opts_gui::actions::UiAction;
use opts_gui::interaction::Activated;
use opts_gui::testing::{act, layout_app, rules};
use opts_gui::ui_tree::{find_entity, snapshot};

fn history_path(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir()
        .join(format!("opts_wizard_{tag}_{}", std::process::id()))
        .join("drive_history.json")
}

/// Pins the process's OMSI root to an empty installation, so the date lookup of the map step
/// (`global.cfg` of the map folder) never sees the host's real install: the fixture maps share
/// folder names with real maps. The first call wins for the whole test binary.
fn pin_fixture_root() {
    let root = std::env::temp_dir().join(format!("opts-wizard-root-{}", std::process::id()));
    std::fs::create_dir_all(root.join("maps")).expect("fixture root");
    let _ = opts_game::omsi_root::locate_omsi_root(Some(&root));
}

fn wizard_with(history: PlannerHistory) -> App {
    pin_fixture_root();
    let mut app = layout_app();
    // The shell's content area: a fixed-size frame, like a 1280x720 window without the rail.
    let frame = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(1280.0),
            height: Val::Px(720.0),
            ..default()
        })
        .id();
    app.insert_resource(opts_game::shell::ShellContent(Some(frame)));
    app.add_plugins(bevy::state::app::StatesPlugin)
        .insert_resource(GameSettings::default())
        .init_state::<AppState>()
        .insert_resource(opts_game::catalogue_service::testing::fixture_lists())
        .insert_resource(history)
        .add_plugins(NewGameMenuPlugin);
    // The shell's one Esc mechanism; the shell's navigation is stood in for by a system that
    // answers `NavigateBack` with Home (the back stack is empty here).
    opts_game::shell::esc::build(&mut app);
    app.add_systems(
        Update,
        |mut back: MessageReader<opts_game::shell::nav::NavigateBack>,
         mut next: ResMut<NextState<AppState>>| {
            if back.read().count() > 0 {
                next.set(AppState::MainMenu);
            }
        },
    );
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    for _ in 0..8 {
        app.update();
    }
    app
}

fn wizard(tag: &str) -> App {
    let _ = std::fs::remove_dir_all(history_path(tag).parent().expect("dir"));
    wizard_with(PlannerHistory::at(Some(history_path(tag))))
}

fn step(app: &App) -> PlannerStep {
    app.world().resource::<PlannerState>().step
}

fn state(app: &App) -> AppState {
    *app.world().resource::<State<AppState>>().get()
}

fn press(app: &mut App, id: &str) {
    let e = find_entity(app.world_mut(), id).unwrap_or_else(|| panic!("no element {id}"));
    app.world_mut().write_message(Activated(e));
    for _ in 0..4 {
        app.update();
    }
}

fn texts(app: &mut App) -> Vec<String> {
    let mut q = app.world_mut().query::<&Text>();
    q.iter(app.world()).map(|t| t.0.clone()).collect()
}

fn cleanup(tag: &str) {
    let _ = std::fs::remove_dir_all(history_path(tag).parent().expect("dir"));
}

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

fn choose_in_what(app: &mut App, id: &str) {
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

#[test]
fn drive_without_a_vehicle_is_a_clear_choice_in_the_what_step() {
    let mut app = wizard("none");
    press(&mut app, "wizard.footer.next");
    assert_eq!(step(&app), PlannerStep::What);
    let t = texts(&mut app);
    assert!(
        t.iter().any(|x| x == "Drive without a vehicle"),
        "a button says it: {t:?}"
    );
    assert!(!app.world().resource::<NewGameSetup>().no_vehicle);
    // The pinned list entry does the same as the button.
    choose_in_what(&mut app, ":none");
    assert!(app.world().resource::<NewGameSetup>().no_vehicle);
    let t = texts(&mut app);
    assert!(
        t.iter().any(|x| x.contains("free camera")),
        "the detail says what happens: {t:?}"
    );
    app.world_mut().resource_mut::<NewGameSetup>().no_vehicle = false;
    choose_in_what(
        &mut app,
        &opts_game::catalogue_service::testing::fixture_lists().vehicles[0].id,
    );
    press(&mut app, "wizard.what.none");
    assert!(app.world().resource::<NewGameSetup>().no_vehicle);
    assert_eq!(step(&app), PlannerStep::When, "on to the next step");
    // Valid without a vehicle; the review says so; Drive starts and remembers it.
    press(&mut app, "wizard.footer.skip");
    assert_eq!(step(&app), PlannerStep::Review);
    let t = texts(&mut app);
    assert!(t.iter().any(|x| x == "No vehicle"), "{t:?}");
    press(&mut app, "wizard.footer.drive");
    assert_eq!(state(&app), AppState::InGame);
    let saved = DriveHistory::load_from(&history_path("none"));
    assert!(saved.last().expect("history").no_vehicle);
    cleanup("none");
}

#[test]
fn choosing_a_vehicle_after_none_turns_the_map_only_start_off() {
    let mut app = wizard("none-off");
    press(&mut app, "wizard.footer.next");
    choose_in_what(&mut app, ":none");
    assert!(app.world().resource::<NewGameSetup>().no_vehicle);
    let id = opts_game::catalogue_service::testing::fixture_lists().vehicles[0]
        .id
        .clone();
    choose_in_what(&mut app, &id);
    let setup = app.world().resource::<NewGameSetup>().clone();
    assert!(!setup.no_vehicle);
    assert!(!setup.selected_vehicle.is_empty());
    cleanup("none-off");
}

#[test]
fn the_last_drive_is_applied_when_the_wizard_opens() {
    let tag = "remember";
    let _ = std::fs::remove_dir_all(history_path(tag).parent().expect("dir"));
    let mut h = PlannerHistory::at(Some(history_path(tag)));
    h.history.record(DriveRecord {
        map: "Grundorf".into(),
        entry_point: "Bauernhof".into(),
        vehicle: "MAN SD200 SD77".into(),
        no_vehicle: false,
        livery: String::new(),
        weather_preset: "Rainy Afternoon".into(),
        year: 1975,
        month: 3,
        day: 4,
        hour: 18,
        minute: 0,
        setvars: Vec::new(),
        last_driven: 9,
    });
    let mut app = wizard_with(h);
    let s = app.world().resource::<NewGameSetup>().clone();
    assert_eq!(s.selected_map, "Grundorf");
    assert_eq!((s.year, s.hour), (1975, 18));
    assert!(
        app.world().resource::<PlannerState>().time_chosen,
        "a remembered drive brings its time: choosing a map keeps it"
    );
    assert_eq!(s.weather_preset, "Rainy Afternoon");
    let t = texts(&mut app);
    assert!(t.iter().any(|x| x.contains("Grundorf")));
    cleanup(tag);
}

#[test]
fn an_invalid_choice_disables_next_with_a_reason() {
    let mut app = wizard("invalid");
    app.world_mut().resource_mut::<NewGameSetup>().selected_map = "Nowhere".into();
    for _ in 0..4 {
        app.update();
    }
    let next = find_entity(app.world_mut(), "wizard.footer.next").expect("next");
    let w = app.world();
    assert!(w
        .get::<opts_gui::interaction::state::Disabled>(next)
        .is_some());
    let reason = w
        .get::<opts_gui::interaction::state::DisabledReason>(next)
        .expect("a disabled control says why");
    assert_eq!(reason.0, "Choose a map first");
    press(&mut app, "wizard.footer.next");
    assert_eq!(
        step(&app),
        PlannerStep::Where,
        "a disabled Next does nothing"
    );
    cleanup("invalid");
}

#[test]
fn facts_without_data_are_not_shown_and_no_na_appears_anywhere() {
    let mut app = wizard("facts");
    press(&mut app, "wizard.footer.next");
    let t = texts(&mut app);
    assert!(
        !t.iter()
            .any(|x| x.contains("N/A") || x.contains("Scrollable")),
        "{t:?}"
    );
    cleanup("facts");
}

#[test]
fn live_weather_is_visible_marked_and_has_no_numbers() {
    let mut app = wizard("live");
    press(&mut app, "wizard.stepper.when");
    assert_eq!(step(&app), PlannerStep::When);
    let live = find_entity(app.world_mut(), "wizard.when.weather.live").expect("live card");
    let w = app.world();
    assert!(w.get::<opts_gui::not_yet::NotAvailableYet>(live).is_some());
    assert!(w
        .get::<opts_gui::interaction::state::Disabled>(live)
        .is_some());
    let t = texts(&mut app);
    assert!(t.iter().any(|x| x == "Feature not available yet!"));
    assert!(!t
        .iter()
        .any(|x| x.contains("°C") || x.contains("km/h") || x.contains('%')));
    cleanup("live");
}

#[test]
fn a_fresh_plan_starts_on_the_maps_default_date() {
    use opts_game::new_game_screen::planner::{default_map_date, setup_date, today, MapEra};
    let mut app = wizard("default-date");
    assert!(!app.world().resource::<PlannerState>().date_chosen);
    // The fixture maps have no `global.cfg`: no offset, no years, so today.
    assert_eq!(
        setup_date(app.world().resource::<NewGameSetup>()),
        default_map_date(today(), &MapEra::default())
    );
    assert!(find_entity(app.world_mut(), "wizard.where.day.plus").is_some());
    cleanup("default-date");
}

#[test]
fn quick_chips_and_steppers_change_the_time_and_date() {
    let mut app = wizard("when");
    // The date is chosen on the map step (one place), the time on step When.
    let month = app.world().resource::<NewGameSetup>().month;
    press(&mut app, "wizard.where.month.minus");
    assert_eq!(
        app.world().resource::<NewGameSetup>().month,
        if month == 1 { 12 } else { month - 1 }
    );
    assert!(app.world().resource::<PlannerState>().date_chosen);
    press(&mut app, "wizard.stepper.when");
    assert!(find_entity(app.world_mut(), "wizard.when.month.minus").is_none());
    press(&mut app, "wizard.when.quick.3");
    let s = app.world().resource::<NewGameSetup>().clone();
    assert_eq!((s.hour, s.minute), (18, 0));
    press(&mut app, "wizard.when.hour.plus");
    assert_eq!(app.world().resource::<NewGameSetup>().hour, 19);
    press(&mut app, "wizard.when.weather.rainy-afternoon");
    assert_eq!(
        app.world().resource::<NewGameSetup>().weather_preset,
        "Rainy Afternoon"
    );
    cleanup("when");
}

#[test]
fn every_step_meets_the_ci_rules() {
    let mut app = wizard("rules");
    for id in [
        "wizard.stepper.where",
        "wizard.stepper.what",
        "wizard.stepper.when",
        "wizard.stepper.review",
    ] {
        press(&mut app, id);
        for _ in 0..4 {
            app.update();
        }
        let small = rules::small_targets(app.world_mut());
        assert!(small.is_empty(), "{id}: {}", small.join("\n"));
        let text = rules::small_text(app.world_mut());
        assert!(text.is_empty(), "{id}: {}", text.join("\n"));
        let dis = rules::disabled_without_reason(app.world_mut());
        assert!(dis.is_empty(), "{id}: {}", dis.join("\n"));
    }
    cleanup("rules");
}

#[test]
fn the_tree_names_the_controls_of_each_step() {
    let mut app = wizard("tree");
    let snap = snapshot(app.world_mut());
    for id in [
        "wizard.where.search",
        "wizard.where.list",
        "wizard.footer.next",
        "wizard.footer.back",
        "wizard.stepper.review",
    ] {
        assert!(snap.find(id).is_some(), "{id} missing");
    }
    let _ = FooterButton::Drive;
    cleanup("tree");
}

#[test]
fn every_step_is_fully_keyboard_operable() {
    let mut app = wizard("keys");
    for id in [
        "wizard.stepper.where",
        "wizard.stepper.what",
        "wizard.stepper.when",
        "wizard.stepper.review",
    ] {
        press(&mut app, id);
        let report = rules::keyboard_traversal(&mut app);
        assert!(
            report.missed.is_empty(),
            "{id}: never focused {:?}",
            report.missed
        );
        // Confirm on a control that changes the step can hide later controls of the walk
        // (the review's Drive); every other control must react.
        let inert: Vec<_> = report
            .inert
            .iter()
            .filter(|i| !i.contains("footer.drive"))
            .collect();
        assert!(inert.is_empty(), "{id}: Confirm did nothing on {inert:?}");
        // The step jumps (stepper buttons) change the screen; return to the step under test.
        press(&mut app, id);
    }
    cleanup("keys");
}

#[test]
fn the_date_and_time_block_fits_a_1024_window_at_100_and_150_percent() {
    for scale in [1.0_f32, 1.5] {
        let mut app = wizard("fit");
        {
            let mut q = app.world_mut().query::<&mut Window>();
            for mut w in q.iter_mut(app.world_mut()) {
                w.resolution.set(1024.0, 640.0);
            }
            app.world_mut().insert_resource(UiScale(scale));
        }
        // The time block sits at the left of step When and must fit the window; the date block
        // sits in the map step's detail pane and must fit that pane.
        for (stepper, name, in_window) in [
            ("wizard.stepper.where", "DateBlock", false),
            ("wizard.stepper.when", "TimeBlock", true),
        ] {
            press(&mut app, stepper);
            for _ in 0..4 {
                app.update();
            }
            let mut q = app.world_mut().query::<(
                Entity,
                &Name,
                &ComputedNode,
                &UiGlobalTransform,
                Option<&ChildOf>,
            )>();
            let rect = |c: &ComputedNode, t: &UiGlobalTransform| {
                (
                    c.size() * c.inverse_scale_factor,
                    t.translation * c.inverse_scale_factor,
                )
            };
            let (block, parent) = q
                .iter(app.world())
                .find(|(_, n, ..)| n.as_str() == name)
                .map(|(_, _, c, t, p)| (rect(c, t), p.map(ChildOf::parent)))
                .expect("the picker block");
            let limit = if in_window {
                1024.0 / scale
            } else {
                let parent = parent.expect("a parent");
                let (_, _, c, t, _) = q.get(app.world(), parent).expect("parent node");
                let (size, centre) = rect(c, t);
                centre.x + size.x / 2.0
            };
            let right = block.1.x + block.0.x / 2.0;
            assert!(
                right <= limit + 1.0,
                "{name} at scale {scale}: right edge {right} beyond {limit}"
            );
            assert!(block.0.x <= 560.0 + 1.0);
        }
        cleanup("fit");
    }
}

/// Source text without its tests.
fn source_of(path: &str) -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    let text = std::fs::read_to_string(p).unwrap_or_default();
    text.split("#[cfg(test)]").next().unwrap_or("").to_string()
}

#[test]
fn the_wizard_and_loading_modules_hold_no_hard_coded_colours_or_visible_literals() {
    let mut files: Vec<String> = std::fs::read_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/new_game_screen"),
    )
    .expect("module")
    .flatten()
    .map(|e| format!("src/new_game_screen/{}", e.file_name().to_string_lossy()))
    .filter(|f| f.ends_with(".rs"))
    .collect();
    files.extend(
        [
            "src/in_game/loading_ui.rs",
            "src/in_game/loading_pipeline.rs",
            "src/in_game/load_report.rs",
        ]
        .map(String::from),
    );
    let mut bad = Vec::new();
    for f in &files {
        for (n, line) in source_of(f).lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            for banned in [
                "Color::srgb",
                "Color::rgb",
                "Color::hsl",
                "Color::BLACK",
                "Color::WHITE",
                "Srgba::",
                "LinearRgba::",
            ] {
                if code.contains(banned) {
                    bad.push(format!("{f}:{}: {}", n + 1, code.trim()));
                }
            }
            if let Some(i) = code.find("Text::new(\"") {
                let lit = &code[i + 11..];
                if lit
                    .chars()
                    .take_while(|c| *c != '"')
                    .filter(|c| c.is_alphabetic())
                    .count()
                    >= 3
                {
                    bad.push(format!("{f}:{}: literal text {}", n + 1, code.trim()));
                }
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// P-1014: a wizard opened while the first scan still runs (empty lists) must show the lists once
/// the catalogue arrives, and leaving it afterwards must not touch despawned entities.
#[test]
fn a_wizard_opened_on_an_empty_catalogue_fills_when_the_lists_arrive() {
    let tag = "late";
    let _ = std::fs::remove_dir_all(history_path(tag).parent().expect("dir"));
    let fixture = opts_game::catalogue_service::testing::fixture_lists();
    let mut app = layout_app();
    let frame = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(1280.0),
            height: Val::Px(720.0),
            ..default()
        })
        .id();
    app.insert_resource(opts_game::shell::ShellContent(Some(frame)));
    app.add_plugins(bevy::state::app::StatesPlugin)
        .insert_resource(GameSettings::default())
        .init_state::<AppState>()
        .insert_resource(opts_game::new_game_screen::GameCatalogue::default())
        .insert_resource(PlannerHistory::at(Some(history_path(tag))))
        .add_plugins(NewGameMenuPlugin);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    for _ in 0..8 {
        app.update();
    }
    let before = texts(&mut app);
    assert!(
        !before.iter().any(|x| x.starts_with("Search 42 maps")),
        "empty catalogue: {before:?}"
    );
    *app.world_mut()
        .resource_mut::<opts_game::new_game_screen::types::GameCatalogue>() = fixture;
    for _ in 0..8 {
        app.update();
    }
    let after = texts(&mut app);
    assert!(
        after.iter().any(|x| x.starts_with("Search 42 maps")),
        "the list was built again: {after:?}"
    );
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    for _ in 0..8 {
        app.update();
    }
    cleanup(tag);
}

// --- M-0070: the start time of day comes from the map, else from the clock -------------------

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

// --- M-0078: the equipment dialog fits its control to each option -----------------------------

#[test]
fn the_equipment_dialog_offers_a_control_that_fits_each_option_and_writes_the_choices() {
    use bevy::ecs::system::RunSystemOnce;
    use opts_game::new_game_screen::livery::LiveryState;
    use opts_game::new_game_screen::options::{
        open_options, OptionChoice, OptionNumber, OptionSwitch,
    };
    use opts_game::new_game_screen::repaints::VehicleRepaints;
    use opts_gui::interaction::events::{ComboboxSelected, SwitchToggled, TextChanged};
    use opts_gui::l10n::Localization;

    let mut app = wizard("options-dialog");
    let mut reps = VehicleRepaints {
        default_name: "Red".into(),
        ..VehicleRepaints::default()
    };
    reps.setvars.insert(
        "Red".into(),
        vec![
            ("vis_x".into(), "1".into()),
            ("display_ext".into(), "2".into()),
            ("vehicle_id".into(), "3964".into()),
        ],
    );
    reps.setvars
        .insert("Blue".into(), vec![("display_ext".into(), "5".into())]);
    app.world_mut().resource_mut::<LiveryState>().repaints = Some(reps);
    app.world_mut()
        .run_system_once(
            |mut commands: Commands,
             loc: Res<Localization>,
             catalogue: Res<opts_game::new_game_screen::GameCatalogue>,
             setup: Res<NewGameSetup>,
             liveries: Res<LiveryState>| {
                open_options(&mut commands, &loc, &catalogue, &setup, &liveries);
            },
        )
        .expect("open the dialog");
    for _ in 0..3 {
        app.update();
    }

    let switch = find_entity(app.world_mut(), "wizard.what.option.vis-x").expect("a switch");
    assert!(app.world().get::<OptionSwitch>(switch).is_some());
    let combo = find_entity(app.world_mut(), "wizard.what.option.display-ext").expect("a choice");
    let choice = app
        .world()
        .get::<OptionChoice>(combo)
        .expect("a choice box")
        .clone();
    assert_eq!(
        choice.values,
        vec![2, 5],
        "what the repaints give; 0 only when the repaint leaves it unset"
    );
    assert_eq!(choice.default, 2);
    let field = find_entity(app.world_mut(), "wizard.what.option.vehicle-id").expect("a number");
    assert!(app.world().get::<OptionNumber>(field).is_some());

    // Choosing the second value (5) writes it; choosing the default again removes the override.
    app.world_mut().write_message(ComboboxSelected {
        entity: combo,
        selected_index: 1,
        selected_value: "5".into(),
    });
    app.update();
    assert_eq!(
        app.world()
            .resource::<NewGameSetup>()
            .custom_setvars
            .get("display_ext"),
        Some(&5)
    );
    app.world_mut().write_message(ComboboxSelected {
        entity: combo,
        selected_index: 0,
        selected_value: "2".into(),
    });
    app.update();
    assert!(!app
        .world()
        .resource::<NewGameSetup>()
        .custom_setvars
        .contains_key("display_ext"));

    // A number: valid text is written, text that is no number or out of range is ignored.
    for (text, want) in [
        ("100", Some(100)),
        ("abc", Some(100)),
        ("-5", Some(100)),
        ("3964", None),
    ] {
        app.world_mut().write_message(TextChanged {
            entity: field,
            text: text.into(),
        });
        app.update();
        assert_eq!(
            app.world()
                .resource::<NewGameSetup>()
                .custom_setvars
                .get("vehicle_id")
                .copied(),
            want,
            "after typing {text:?}"
        );
    }

    // The switch writes 0 against its default 1.
    app.world_mut().write_message(SwitchToggled {
        entity: switch,
        is_on: false,
    });
    app.update();
    assert_eq!(
        app.world()
            .resource::<NewGameSetup>()
            .custom_setvars
            .get("vis_x"),
        Some(&0)
    );
    cleanup("options-dialog");
}
