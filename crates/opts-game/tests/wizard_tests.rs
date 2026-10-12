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

fn wizard_with(history: PlannerHistory) -> App {
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

#[test]
fn the_last_drive_is_applied_when_the_wizard_opens() {
    let tag = "remember";
    let _ = std::fs::remove_dir_all(history_path(tag).parent().expect("dir"));
    let mut h = PlannerHistory::at(Some(history_path(tag)));
    h.history.record(DriveRecord {
        map: "Grundorf".into(),
        entry_point: "Bauernhof".into(),
        vehicle: "MAN SD200 SD77".into(),
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
fn quick_chips_and_steppers_change_the_time_and_date() {
    let mut app = wizard("when");
    press(&mut app, "wizard.stepper.when");
    press(&mut app, "wizard.when.quick.3");
    let s = app.world().resource::<NewGameSetup>().clone();
    assert_eq!((s.hour, s.minute), (18, 0));
    press(&mut app, "wizard.when.hour.plus");
    assert_eq!(app.world().resource::<NewGameSetup>().hour, 19);
    press(&mut app, "wizard.when.month.minus");
    assert_eq!(app.world().resource::<NewGameSetup>().month, 9);
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
        press(&mut app, "wizard.stepper.when");
        for _ in 0..4 {
            app.update();
        }
        let width = 1024.0 / scale;
        let mut q = app
            .world_mut()
            .query::<(&Name, &ComputedNode, &UiGlobalTransform)>();
        let block = q
            .iter(app.world())
            .find(|(n, ..)| n.as_str() == "DateTimeBlock")
            .map(|(_, c, t)| {
                (
                    c.size() * c.inverse_scale_factor,
                    t.translation * c.inverse_scale_factor,
                )
            })
            .expect("the picker block");
        let right = block.1.x + block.0.x / 2.0;
        assert!(
            right <= width + 1.0,
            "scale {scale}: right edge {right} beyond {width}"
        );
        assert!(block.0.x <= 560.0 + 1.0);
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
