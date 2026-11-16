//! The stage sentence and the manifest texts of the loading screen.

use super::*;

/// The stage sentence for the state: its key and variables.
pub fn stage_message(state: &SimulationLoadState) -> (&'static str, L10nArgs) {
    match state.stage {
        LoadStage::InitEngine if state.waiting_for_catalogue => {
            ("load-stage-catalogue", L10nArgs::default())
        }
        LoadStage::InitEngine => ("load-stage-resolve", L10nArgs::default()),
        LoadStage::BuildTerrain => (
            "load-stage-map",
            L10nArgs(vec![
                ("done".into(), state.tiles_done.into()), // l10n-ok: variable name
                ("total".into(), state.tiles_total.into()), // l10n-ok: variable name
            ]),
        ),
        LoadStage::SetupAtmosphere => ("load-stage-light", L10nArgs::default()),
        LoadStage::SpawnVehicle => ("load-stage-vehicle", L10nArgs::default()),
        LoadStage::CalibrateControls => ("load-stage-start", L10nArgs::default()),
        LoadStage::Ready => ("load-stage-ready", L10nArgs::default()),
        LoadStage::Failed => ("load-stage-failed", L10nArgs::default()),
    }
}

/// The manifest texts of the session that is loading.
pub fn manifest_lines(
    loc: &Localization,
    weather: &opts_bevy::WeatherCatalogue,
    session: &InGameSessionInfo,
    setup: &NewGameSetup,
) -> [(Tile, [String; 2]); 3] {
    let when = NewGameSetup {
        hour: session.start_hour,
        minute: session.start_minute,
        weather_preset: session.weather_preset.clone(),
        ..setup.clone()
    };
    [
        (
            Tile::Map,
            [session.map_name.clone(), session.entry_point.clone()],
        ),
        (
            Tile::Vehicle,
            if session.vehicle_none {
                [loc.t("wiz-no-vehicle"), loc.t("load-no-vehicle-sub")]
            } else {
                [session.vehicle_name.clone(), session.livery_name.clone()]
            },
        ),
        (
            Tile::When,
            [
                crate::new_game_screen::texts::date_text(loc, &when),
                format!(
                    "{} · {}",
                    crate::new_game_screen::planner::clock_text(&when),
                    crate::new_game_screen::texts::weather_name(loc, weather, &when)
                ),
            ],
        ),
    ]
}
