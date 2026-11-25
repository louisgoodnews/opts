//! Registration of settings on the app and the plugin that wires the settings' systems.

use bevy::prelude::*;
use opts_gui::l10n::Localization;

use super::{
    add_messages, builtin, display, lighting, live, protocol, protocol_confirm, SettingChanged,
    SettingDescriptor, SettingsCapabilities, SettingsRegistry,
};
use crate::settings::GameSettings;

/// Registration on the [`App`].
pub trait SettingsAppExt {
    /// Registers one setting.
    fn register_setting(&mut self, d: SettingDescriptor) -> &mut Self;
    /// Registers several settings.
    fn register_settings(&mut self, ds: &[SettingDescriptor]) -> &mut Self;
}

impl SettingsAppExt for App {
    fn register_setting(&mut self, d: SettingDescriptor) -> &mut Self {
        self.init_resource::<SettingsRegistry>();
        self.world_mut()
            .resource_mut::<SettingsRegistry>()
            .register(d);
        self
    }

    fn register_settings(&mut self, ds: &[SettingDescriptor]) -> &mut Self {
        for d in ds {
            self.register_setting(*d);
        }
        self
    }
}

/// Adds the registry, the message and the migration of renamed keys.
pub struct SettingsRegistryPlugin;

impl Plugin for SettingsRegistryPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<SettingsRegistry>() {
            app.insert_resource(SettingsRegistry::with_builtin());
        } else {
            let mut r = app.world_mut().resource_mut::<SettingsRegistry>();
            for d in builtin::all() {
                r.register(d);
            }
        }
        if !app.world().contains_resource::<Localization>() {
            app.insert_resource(Localization::detect());
        }
        add_messages(&mut app.world_mut().resource_mut::<Localization>());
        if !app.is_plugin_added::<lighting::LightQualityGamePlugin>() {
            app.add_plugins(lighting::LightQualityGamePlugin);
        }
        app.init_resource::<crate::audio_content::ClipCache>()
            .init_resource::<SettingsCapabilities>()
            .add_message::<SettingChanged>()
            .insert_resource(display::DisplayBackend(Box::new(display::WindowDisplay)))
            .add_systems(
                Startup,
                (migrate_registered, display::apply_saved_display).chain(),
            )
            .add_systems(
                Update,
                (
                    display::apply_display_changes,
                    live::apply_live_changes,
                    live::sync_developer_panel,
                    live::sync_water,
                    live::sync_tile_loading,
                    protocol_confirm::tick_protocol_revert
                        .run_if(resource_exists::<protocol_confirm::ProtocolRevert>),
                    live::sync_hdr,
                    live::sync_view_distance,
                    live::sync_detail_distances,
                    live::sync_shadow_detail,
                    live::sync_audio,
                    crate::vehicle_setup_settings::sync_vehicle_setup,
                    crate::vehicle_fuel_settings::sync_fuel_mode,
                    live::sync_weather_effects,
                    live::sync_tree_wind,
                ),
            )
            .add_systems(
                Startup,
                (
                    crate::language_packs::load_startup,
                    live::apply_saved_language,
                    live::apply_saved_trace,
                )
                    .chain(),
            )
            .add_systems(Update, crate::language_packs::reload_action)
            .add_systems(Update, crate::head_track_settings::sync_head_track_settings);
        protocol::register(app);
    }
}

fn migrate_registered(registry: Res<SettingsRegistry>, mut settings: ResMut<GameSettings>) {
    let done = registry.migrate(&mut settings);
    if !done.is_empty() {
        tracing::info!(keys = ?done, "migrated renamed settings");
    }
}
