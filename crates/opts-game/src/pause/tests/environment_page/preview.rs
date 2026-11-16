//! Time, weather and the installed catalogue on the environment page.

use super::*;
use opts_bevy::CalendarDate;

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
