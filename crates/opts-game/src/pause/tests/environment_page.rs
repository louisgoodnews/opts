//! The environment page of the pause overlay.

use super::*;
use opts_bevy::{
    CalendarDate, ChronoRebuild, EnvironmentChange, MapDateChanged, SessionEnvironment,
};

fn drive_at_noon() -> App {
    let mut app = app();
    app.world_mut().write_message(EnvironmentChange::reset(
        CalendarDate::new(1994, 6, 1),
        12.0 * 3600.0,
        "Clear Summer Day",
    ));
    app.update();
    app
}

fn env(app: &App) -> SessionEnvironment {
    app.world().resource::<SessionEnvironment>().clone()
}

fn open_page(app: &mut App) {
    esc(app);
    click(app, "pause.environment");
    let ids = ids(app);
    assert!(
        ids.contains(&"pause.environment.apply".to_string()),
        "{ids:?}"
    );
    assert!(
        !ids.contains(&"pause.resume".to_string()),
        "one dialog at a time"
    );
}

fn light_row_text(app: &mut App) -> String {
    let kids: Vec<Entity> = app
        .world_mut()
        .query_filtered::<&Children, With<light_row::LightModeButton>>()
        .iter(app.world())
        .flat_map(|c| c.iter().collect::<Vec<_>>())
        .collect();
    kids.iter()
        .filter_map(|k| app.world().get::<Text>(*k).map(|t| t.0.clone()))
        .collect()
}

#[test]
fn the_lighting_quality_row_shows_the_mode_and_level_and_toggles_it() {
    use opts_bevy::light_quality::{GovernorState, LightMode, LightQuality};
    let mut app = drive_at_noon();
    open_page(&mut app);
    app.update();
    assert_eq!(light_row_text(&mut app), "Automatic (level 0: full)");
    app.world_mut().resource_mut::<GovernorState>().level = 2;
    app.update();
    app.update();
    assert_eq!(
        light_row_text(&mut app),
        "Automatic (level 2: headlights only)"
    );
    click(&mut app, "pause.environment.light-quality");
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(light_row_text(&mut app), "Custom");
    let q = *app.world().resource::<LightQuality>();
    assert_eq!((q.mode, q.level), (LightMode::Custom, 2));
    click(&mut app, "pause.environment.light-quality");
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(light_row_text(&mut app), "Automatic (level 0: full)");
    assert!(paused(&app), "the page still pauses the drive");
}

#[test]
fn time_and_weather_preview_live_and_esc_restores_them() {
    let mut app = drive_at_noon();
    open_page(&mut app);
    click(&mut app, "pause.environment.hour.plus");
    click(&mut app, "pause.environment.weather.rainy-afternoon");
    click(&mut app, "pause.environment.day.plus");
    let e = env(&app);
    assert_eq!(e.hms().0, 13, "the time changes at once");
    assert_eq!(
        e.weather(),
        "Rainy Afternoon",
        "the weather changes at once"
    );
    assert_eq!(
        e.date(),
        CalendarDate::new(1994, 6, 1),
        "the date waits for Apply"
    );
    assert!(paused(&app));
    esc(&mut app);
    let e = env(&app);
    assert_eq!(e.hms(), (12, 0, 0));
    assert_eq!(e.weather(), "Clear Summer Day");
    assert_eq!(e.date(), CalendarDate::new(1994, 6, 1));
    assert!(!app.world().resource::<environment::EnvironmentPage>().open);
    let ids = ids(&mut app);
    assert!(
        ids.contains(&"pause.resume".to_string()),
        "back in the overlay"
    );
    assert!(paused(&app));
}

fn installed_catalogue() -> opts_bevy::WeatherCatalogue {
    use opts_core::formats::weather::{PresetFile, WeatherFile, WeatherFolder};
    let preset = |id: &str| PresetFile {
        id: id.into(),
        weather: WeatherFile::default(),
        texts: Default::default(),
    };
    opts_bevy::WeatherCatalogue::from_folder(
        "/omsi".into(),
        WeatherFolder {
            presets: vec![preset("#CAVOK"), preset("Sturm")],
            ..default()
        },
    )
}

#[test]
fn an_installed_catalogue_lists_its_presets_and_an_old_id_marks_its_stand_in() {
    use opts_gui::interaction::state::Selected;
    let mut app = drive_at_noon();
    *app.world_mut()
        .resource_mut::<opts_bevy::WeatherCatalogue>() = installed_catalogue();
    app.update();
    open_page(&mut app);
    let listed = ids(&mut app);
    assert!(
        listed.contains(&"pause.environment.weather.sturm".to_string()),
        "{listed:?}"
    );
    assert!(listed.contains(&"pause.environment.weather.cavok".to_string()));
    assert!(
        !listed.contains(&"pause.environment.weather.rainy-afternoon".to_string()),
        "the built-in presets are not listed beside the install's"
    );
    // The session still has the old id; its chip is the stand-in (`#CAVOK`).
    assert_eq!(env(&app).weather(), "Clear Summer Day");
    let selected = |app: &mut App, id: &str| {
        let e = opts_gui::ui_tree::find_entity(app.world_mut(), id).expect(id);
        app.world().entity(e).contains::<Selected>()
    };
    assert!(selected(&mut app, "pause.environment.weather.cavok"));
    assert!(!selected(&mut app, "pause.environment.weather.sturm"));
    click(&mut app, "pause.environment.weather.sturm");
    assert_eq!(env(&app).weather(), "Sturm");
    assert!(selected(&mut app, "pause.environment.weather.sturm"));
    assert!(!selected(&mut app, "pause.environment.weather.cavok"));
}

#[test]
fn apply_sets_the_date_and_announces_the_map_date() {
    let mut app = drive_at_noon();
    open_page(&mut app);
    click(&mut app, "pause.environment.day.plus");
    click(&mut app, "pause.environment.year.minus");
    click(&mut app, "pause.environment.minute.plus");
    click(&mut app, "pause.environment.apply");
    let e = env(&app);
    assert_eq!(e.date(), CalendarDate::new(1993, 6, 2));
    assert_eq!(e.map_date(), CalendarDate::new(1993, 6, 2));
    assert_eq!(e.hms(), (12, 5, 0));
    let msgs = app.world().resource::<Messages<MapDateChanged>>();
    let mut cursor = msgs.get_cursor();
    let got: Vec<_> = cursor.read(msgs).copied().collect();
    assert_eq!(
        got,
        vec![MapDateChanged {
            old: CalendarDate::new(1994, 6, 1),
            new: CalendarDate::new(1993, 6, 2)
        }]
    );
    assert!(ids(&mut app).contains(&"pause.resume".to_string()));
}

#[test]
fn resume_waits_for_a_map_rebuild() {
    let mut app = drive_at_noon();
    esc(&mut app);
    app.world_mut()
        .resource_mut::<ChronoRebuild>()
        .pending_tiles = 3;
    click(&mut app, "pause.resume");
    assert!(!app.world().resource::<Pause>().open);
    assert!(paused(&app), "still rebuilding");
    app.world_mut()
        .resource_mut::<ChronoRebuild>()
        .pending_tiles = 0;
    app.update();
    assert!(!paused(&app));
}
