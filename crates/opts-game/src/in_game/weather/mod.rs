//! Weather in the running drive: the preset catalogue read from the install, and the effects
//! that follow the [`SessionEnvironment`](opts_bevy::SessionEnvironment): distance fog, the cloud layer, rain and snow, wet and
//! snowy ground. Everything reads `SessionEnvironment::conditions()`, so a weather change (an
//! `EnvironmentChange`) takes effect in the next frame; the pure parameter functions are in
//! `opts_bevy::weather`. Design and budget: `docs/src/architecture/weather.md`.

use bevy::prelude::*;
use opts_bevy::{EnvironmentSet, WeatherCatalogue, WeatherEffects};

use crate::app_state::AppState;

pub mod clouds;
pub mod fog;
pub mod precip;
pub mod puddles;
pub mod surfaces;

/// Seconds between attempts to read the install's `Weather` folder (until it was read).
const CATALOGUE_RETRY_S: f32 = 2.0;

/// Reads the install's presets as soon as the install is known; the catalogue keeps the built-in
/// presets until then. A preset id already chosen resolves again when the catalogue arrives
/// (`opts_bevy::environment::refresh_weather_on_catalogue_change`).
pub fn refresh_weather_catalogue(
    time: Res<Time<Real>>,
    mut catalogue: ResMut<WeatherCatalogue>,
    mut next: Local<f32>,
) {
    if catalogue.is_installed() || time.elapsed_secs() < *next {
        return;
    }
    *next = time.elapsed_secs() + CATALOGUE_RETRY_S;
    if let Some(root) = crate::omsi_root::known_omsi_root() {
        if let Some(read) = WeatherCatalogue::read(&root) {
            tracing::info!(
                presets = read.ids().len(),
                root = %root.display(),
                "weather presets read from the install"
            );
            *catalogue = read;
        }
    }
}

/// Registers the weather effects of a drive.
#[derive(Default)]
pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeatherCatalogue>()
            .init_resource::<WeatherEffects>()
            .init_resource::<opts_bevy::weather::GroundFogView>()
            .init_resource::<opts_bevy::reflections::Wetness>()
            .init_resource::<clouds::CloudState>()
            .init_resource::<precip::PrecipState>()
            .init_resource::<puddles::PuddleState>()
            .add_systems(Update, refresh_weather_catalogue.before(EnvironmentSet))
            .add_systems(
                Update,
                fog::measure_fog_view
                    .after(EnvironmentSet)
                    .before(super::sky::update_sky)
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                (
                    fog::update_fog.after(fog::measure_fog_view),
                    clouds::update_clouds,
                    precip::update_precipitation,
                    puddles::update_puddles.after(surfaces::drive_wetness),
                    (surfaces::drive_wetness, surfaces::update_surfaces).chain(),
                )
                    .after(EnvironmentSet)
                    .after(super::sky::update_sky)
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::in_game::types::InGameCamera;
    use bevy::pbr::DistanceFog;
    use opts_bevy::weather::PrecipitationQuality;
    use opts_bevy::{CalendarDate, EnvironmentChange, EnvironmentPlugin, SessionEnvironment};
    use opts_core::formats::weather::PrecipKind;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), EnvironmentPlugin))
            .init_asset::<Image>()
            .init_asset::<Mesh>()
            .init_asset::<StandardMaterial>()
            .init_asset::<opts_bevy::ground_material::GroundMaterial>()
            .init_resource::<opts_bevy::reflections::Wetness>()
            .init_resource::<opts_bevy::weather::GroundFogView>()
            .init_resource::<clouds::CloudState>()
            .init_resource::<precip::PrecipState>()
            .add_systems(
                Update,
                (
                    fog::measure_fog_view,
                    fog::update_fog.after(fog::measure_fog_view),
                    clouds::update_clouds,
                    precip::update_precipitation,
                    (surfaces::drive_wetness, surfaces::update_surfaces).chain(),
                )
                    .after(EnvironmentSet),
            );
        app.world_mut()
            .spawn((InGameCamera, GlobalTransform::default()));
        app.world_mut().write_message(EnvironmentChange::reset(
            CalendarDate::new(2026, 6, 29),
            43_200.0,
            "Clear Summer Day",
        ));
        app.update();
        app
    }

    fn set_weather(app: &mut App, id: &str) {
        app.world_mut().write_message(EnvironmentChange {
            weather: Some(id.into()),
            ..default()
        });
        app.update();
        app.update();
    }

    fn camera_has_fog(app: &mut App) -> bool {
        let mut q = app
            .world_mut()
            .query_filtered::<Has<DistanceFog>, With<InGameCamera>>();
        q.single(app.world()).unwrap()
    }

    fn precip_visible(app: &mut App) -> bool {
        let mut q = app
            .world_mut()
            .query_filtered::<&Visibility, With<precip::PrecipLayer>>();
        q.iter(app.world()).any(|v| *v == Visibility::Visible)
    }

    #[test]
    fn a_weather_change_updates_fog_and_precipitation_live() {
        let mut app = app();
        assert!(!camera_has_fog(&mut app), "clear air has no fog");
        assert!(!precip_visible(&mut app));

        set_weather(&mut app, "Autumn Morning Fog");
        assert!(camera_has_fog(&mut app));

        set_weather(&mut app, "Rainy Afternoon");
        assert!(camera_has_fog(&mut app), "6 km visibility is hazy");
        let state = app.world().resource::<precip::PrecipState>();
        assert_eq!(state.kind(), Some(PrecipKind::Rain));
        assert!(state.drawn() > 0 && state.drawn() <= state.allocated());
        assert!(precip_visible(&mut app));
        let rain_quads = app.world().resource::<precip::PrecipState>().drawn();

        set_weather(&mut app, "Winter Snowfall");
        let state = app.world().resource::<precip::PrecipState>();
        assert_eq!(state.kind(), Some(PrecipKind::Snow));
        assert!(state.drawn() > 0);
        assert_ne!(state.drawn(), 0, "{rain_quads}");

        set_weather(&mut app, "Clear Summer Day");
        assert!(!camera_has_fog(&mut app));
        assert!(!precip_visible(&mut app));
        assert_eq!(app.world().resource::<precip::PrecipState>().kind(), None);
    }

    #[test]
    fn the_settings_switch_each_effect_off_live() {
        let mut app = app();
        set_weather(&mut app, "Rainy Afternoon");
        assert!(camera_has_fog(&mut app) && precip_visible(&mut app));

        app.world_mut().resource_mut::<WeatherEffects>().fog = false;
        app.update();
        assert!(!camera_has_fog(&mut app));

        app.world_mut()
            .resource_mut::<WeatherEffects>()
            .precipitation = PrecipitationQuality::Off;
        app.update();
        assert!(!precip_visible(&mut app));

        // Back on, with a smaller budget: fewer quads than the medium one.
        app.world_mut()
            .resource_mut::<WeatherEffects>()
            .precipitation = PrecipitationQuality::Medium;
        app.update();
        let medium = app.world().resource::<precip::PrecipState>().drawn();
        app.world_mut()
            .resource_mut::<WeatherEffects>()
            .precipitation = PrecipitationQuality::Low;
        app.update();
        let low = app.world().resource::<precip::PrecipState>().drawn();
        assert!(low > 0 && low < medium, "{low} < {medium}");
        assert!(low <= PrecipitationQuality::Low.budget() as usize);
    }

    #[test]
    fn the_weather_drives_the_wetness_resource_and_the_setting_dries_it() {
        use opts_bevy::reflections::Wetness;
        let mut app = app();
        assert_eq!(app.world().resource::<Wetness>().0, 0.0);
        set_weather(&mut app, "Rainy Afternoon");
        assert!(app.world().resource::<Wetness>().0 > 0.9);
        app.world_mut().resource_mut::<WeatherEffects>().wet_ground = false;
        app.update();
        assert_eq!(app.world().resource::<Wetness>().0, 0.0);
    }

    #[test]
    fn the_session_environment_carries_the_conditions_the_effects_read() {
        let mut app = app();
        set_weather(&mut app, "Rainy Afternoon");
        let env = app.world().resource::<SessionEnvironment>();
        assert_eq!(env.conditions().precip, PrecipKind::Rain);
        assert!(env.conditions().fog_visibility_m < 10_000.0);
    }

    #[test]
    fn scenery_and_vehicle_scripts_get_the_weather_values() {
        use opts_bevy::SceneryClock;
        let mut app = app();
        app.init_resource::<SceneryClock>().add_systems(
            Update,
            crate::in_game::omsi_world::sync_scenery_clock.after(EnvironmentSet),
        );
        set_weather(&mut app, "Rainy Afternoon");
        let clock = app.world().resource::<SceneryClock>();
        assert_eq!(clock.weather.precip_type, 1.0);
        assert!(
            clock.weather.precip_rate > 0.4,
            "{}",
            clock.weather.precip_rate
        );
        assert!(clock.weather.abs_hum > 5.0);
        assert_eq!(clock.temperature_c, 15.0);
        set_weather(&mut app, "Winter Snowfall");
        let clock = app.world().resource::<SceneryClock>();
        assert_eq!(
            (clock.weather.precip_type, clock.temperature_c),
            (2.0, -3.0)
        );
        set_weather(&mut app, "Clear Summer Day");
        assert_eq!(
            app.world().resource::<SceneryClock>().weather.precip_type,
            0.0
        );
        // A vehicle's scripts read them as system and local variables.
        let (mut engine, _) =
            opts_sim::script::ScriptEngine::from_sources(&[("weather.osc", "{frame}\n")]);
        set_weather(&mut app, "Rainy Afternoon");
        let clock = app.world().resource::<SceneryClock>().clone();
        opts_bevy::vehicle_script::feed_weather(&mut engine, clock.temperature_c, &clock.weather);
        assert_eq!(engine.get("preciptype"), 1.0);
        assert!((engine.get("preciprate") - 173.0 / 255.0).abs() < 0.3);
    }

    fn fog_density_of(app: &mut App) -> f32 {
        let mut q = app
            .world_mut()
            .query_filtered::<&DistanceFog, With<InGameCamera>>();
        match q.single(app.world()).unwrap().falloff {
            bevy::pbr::FogFalloff::ExponentialSquared { density } => density,
            ref other => panic!("{other:?}"),
        }
    }

    fn place_camera(app: &mut App, y: f32, pitch_deg: f32) {
        let mut q = app
            .world_mut()
            .query_filtered::<&mut GlobalTransform, With<InGameCamera>>();
        let mut tf = q.single_mut(app.world_mut()).unwrap();
        *tf = GlobalTransform::from(
            Transform::from_xyz(0.0, y, 0.0)
                .with_rotation(Quat::from_rotation_x(pitch_deg.to_radians())),
        );
        app.update();
    }

    #[test]
    fn ground_fog_follows_the_camera_height_and_other_presets_do_not() {
        use opts_bevy::weather::GroundFogView;
        let mut app = app();
        set_weather(&mut app, "Autumn Morning Fog");
        // 300 m dusk-like fog: no layer, the same density at any height.
        place_camera(&mut app, 2.0, 0.0);
        let flat = fog_density_of(&mut app);
        place_camera(&mut app, 70.0, 0.0);
        assert_eq!(fog_density_of(&mut app), flat);
        assert_eq!(app.world().resource::<GroundFogView>().layer_m, None);

        // An installed ground fog (75 m, like `Bodennebel`) is a layer.
        let mut w = opts_core::formats::weather::WeatherFile::default();
        w.fog.visibility_m = 75.0;
        w.fog.density = 1.0;
        let folder = opts_core::formats::weather::WeatherFolder {
            presets: vec![opts_core::formats::weather::PresetFile {
                id: "Bodennebel".into(),
                weather: w,
                texts: Default::default(),
            }],
            clouds: Default::default(),
        };
        app.insert_resource(WeatherCatalogue::from_folder("/nonexistent".into(), folder));
        set_weather(&mut app, "Bodennebel");
        place_camera(&mut app, 2.0, 0.0);
        let low = fog_density_of(&mut app);
        place_camera(&mut app, 20.0, 0.0);
        let mid = fog_density_of(&mut app);
        place_camera(&mut app, 70.0, 0.0);
        let high = fog_density_of(&mut app);
        assert!(low > mid && mid > high, "{low} {mid} {high}");
        let view = *app.world().resource::<GroundFogView>();
        assert_eq!(view.layer_m, Some(30.0));
        assert!(view.sky_clear > 0.99 && view.effective_visibility_m > 170.0);
        // Looking up from above the layer clears the fog further; at the ground nothing changes.
        place_camera(&mut app, 70.0, 20.0);
        assert!(fog_density_of(&mut app) < high);
        place_camera(&mut app, 2.0, 20.0);
        assert_eq!(fog_density_of(&mut app), low);

        // The clear preset has no fog at all, at any height.
        set_weather(&mut app, "Clear Summer Day");
        place_camera(&mut app, 70.0, 0.0);
        assert!(!camera_has_fog(&mut app));
    }
}
