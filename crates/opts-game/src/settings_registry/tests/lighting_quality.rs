//! The lighting-quality settings.

use super::*;
use opts_bevy::light_quality::{
    ladder_preset, GovernorState, LightMode, LightQuality, LightQualityInput, Shafts,
};

fn app() -> App {
    crate::isolation::isolate_user_dirs();
    let mut app = App::new();
    app.insert_resource(SettingsRegistry::with_builtin())
        .insert_resource(SettingsCapabilities::default())
        .insert_resource(GameSettings::default())
        .init_resource::<Time<bevy::time::Real>>()
        .add_message::<SettingChanged>()
        .add_plugins(lighting::LightQualityGamePlugin);
    app.update();
    app
}

fn set(app: &mut App, key: &str, value: &str) {
    apply_change(app.world_mut(), key, Some(value), ChangeSource::Protocol)
        .unwrap_or_else(|e| panic!("{key}={value}: {e:?}"));
}

fn quality(app: &App) -> LightQuality {
    *app.world().resource::<LightQuality>()
}

mod gate;
mod keys;
mod modes;

