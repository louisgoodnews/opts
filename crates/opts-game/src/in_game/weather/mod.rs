//! Weather in the running drive: the preset catalogue read from the install, and the effects
//! that follow the [`SessionEnvironment`](opts_bevy::SessionEnvironment): distance fog, the cloud layer, rain and snow, wet and
//! snowy ground. Everything reads `SessionEnvironment::conditions()`, so a weather change (an
//! `EnvironmentChange`) takes effect in the next frame; the pure parameter functions are in
//! `opts_bevy::weather`. Design and budget: `docs/src/architecture/weather.md`.

use bevy::prelude::*;
use opts_bevy::{ArticulatedChain, EnvironmentSet, WeatherCatalogue, WeatherEffects};

use crate::app_state::AppState;
use crate::in_game::omsi_world::VehicleComposition;
use crate::settings::GameSettings;
use crate::settings_registry::lighting::{exhaust_cap_of, exhaust_on_of};

pub mod clouds;
pub use opts_exhaust as exhaust;
pub mod fog;
pub mod grip;
pub mod precip;
pub mod puddles;
pub mod surfaces;

/// The clock, the session environment and the weather effects: what every weather system reads.
#[derive(bevy::ecs::system::SystemParam)]
pub struct WeatherScene<'w> {
    pub(crate) time: Res<'w, Time<Real>>,
    pub(crate) env: Res<'w, opts_bevy::SessionEnvironment>,
    pub(crate) effects: Res<'w, WeatherEffects>,
}

/// The asset stores the weather layers build their meshes, textures and materials in.
#[derive(bevy::ecs::system::SystemParam)]
pub struct WeatherAssets<'w> {
    pub(crate) images: ResMut<'w, Assets<Image>>,
    pub(crate) meshes: ResMut<'w, Assets<Mesh>>,
    pub(crate) materials: ResMut<'w, Assets<StandardMaterial>>,
}

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

/// The vehicles with an articulated chain or a rail consist, with each one's chain and consist.
type RearVehicles<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Option<&'static ArticulatedChain>,
        Option<&'static VehicleComposition>,
    ),
    Or<(With<ArticulatedChain>, With<VehicleComposition>)>,
>;

/// Copies the exhaust settings into the exhaust crate each frame (they are read from the settings
/// registry, which lives in the game), and the units behind each vehicle with a chain or a consist.
fn sync_exhaust(
    settings: Res<GameSettings>,
    mut out: ResMut<exhaust::ExhaustSettings>,
    mut rear: ResMut<exhaust::RearUnits>,
    vehicles: RearVehicles,
) {
    out.on = exhaust_on_of(&settings);
    out.cap = exhaust_cap_of(&settings);
    rear.0 = vehicles
        .iter()
        .map(|(e, chain, rail)| (e, behind(chain, rail)))
        .collect();
}

/// The units behind a vehicle, front to rear: its articulated chain's segments, else a rail
/// consist's followers (ISS-038). The exhaust crate takes the rear-most of them with an emitter.
#[must_use]
pub fn behind(chain: Option<&ArticulatedChain>, rail: Option<&VehicleComposition>) -> Vec<Entity> {
    match (chain, rail) {
        (Some(chain), _) => chain.segments.clone(),
        (None, Some(rail)) => rail.followers.iter().map(|p| p.entity).collect(),
        (None, None) => Vec::new(),
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
            .init_resource::<exhaust::ExhaustState>()
            .init_resource::<exhaust::ExhaustSettings>()
            .init_resource::<exhaust::RearUnits>()
            .add_systems(Update, refresh_weather_catalogue.before(EnvironmentSet))
            .add_systems(Update, sync_exhaust.before(exhaust::update_exhaust))
            .add_systems(Update, exhaust::update_exhaust.after(EnvironmentSet))
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
                    grip::drive_road_grip.after(surfaces::drive_wetness),
                )
                    .after(EnvironmentSet)
                    .after(super::sky::update_sky)
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

#[cfg(test)]
mod tests;
