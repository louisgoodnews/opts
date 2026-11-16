//! The systems that keep the lighting quality and the governor in step with the settings, and the
//! switch from automatic to custom.

use super::*;

/// Keeps [`LightQualityInput`] equal to the settings, live.
pub fn sync_light_input(settings: Res<GameSettings>, mut input: ResMut<LightQualityInput>) {
    let want = LightQualityInput {
        mode: mode_of(&settings),
        custom: custom_of(&settings),
    };
    if *input != want {
        *input = want;
    }
}

/// Keeps the governor's target frame rate equal to the setting, live.
pub fn sync_governor_target(settings: Res<GameSettings>, mut fps: ResMut<GovernorTargetFps>) {
    let want = target_fps_of(&settings);
    if fps.0 != want {
        fps.0 = want;
    }
}

/// Set when the mode just changed from automatic to custom ([`copy_level_to_custom`] acts on it).
#[derive(Resource, Debug, Default)]
pub struct CopyLevelPending(bool);

/// Notices the switch from automatic to custom.
pub fn notice_switch_to_custom(
    mut changes: MessageReader<SettingChanged>,
    mut pending: ResMut<CopyLevelPending>,
) {
    if changes
        .read()
        .any(|c| c.key == MODE_KEY && c.old != "custom" && c.new == "custom")
    {
        pending.0 = true;
    }
}

/// Automatic to custom: "keeps the level reached as starting values" (the custom controls take
/// the values the governor applied). Custom to automatic needs nothing here: the governor
/// restarts at level 0 when it sees the mode change.
pub fn copy_level_to_custom(
    mut pending: ResMut<CopyLevelPending>,
    quality: Res<LightQuality>,
    mut api: SettingsApi,
) {
    if !std::mem::take(&mut pending.0) {
        return;
    }
    let level = ladder_preset(quality.level);
    for (key, text) in CUSTOM_KEYS.iter().zip(texts_of(&level)) {
        let _ = api.change(key, Some(&text), ChangeSource::Protocol, true);
    }
    tracing::info!(
        level = quality.level,
        "lighting quality: custom controls start from the automatic level"
    );
}

/// Opens the governor only in a drive that is running: not paused, not loading.
pub fn update_governor_gate(
    state: Option<Res<State<crate::app_state::AppState>>>,
    time: Option<Res<Time<Virtual>>>,
    load: Option<Res<crate::in_game::types::SimulationLoadState>>,
    loading_screen: Query<(), With<crate::in_game::types::LoadingScreenRoot>>,
    rebuild: Option<Res<opts_bevy::ChronoRebuild>>,
    mut gate: ResMut<GovernorGate>,
) {
    let open = state.is_some_and(|s| *s.get() == crate::app_state::AppState::InGame)
        && time.is_some_and(|t| !t.is_paused())
        && load.is_none_or(|l| l.is_complete)
        && loading_screen.is_empty()
        && !rebuild.is_some_and(|r| r.is_pending());
    if gate.open != open {
        gate.open = open;
    }
}

/// Registers the lighting quality state, the governor and the glue to the settings.
pub struct LightQualityGamePlugin;

impl Plugin for LightQualityGamePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<opts_bevy::light_quality::LightQualityPlugin>() {
            app.add_plugins(opts_bevy::light_quality::LightQualityPlugin);
        }
        app.init_resource::<CopyLevelPending>().add_systems(
            Update,
            ((
                notice_switch_to_custom,
                copy_level_to_custom,
                sync_light_input,
                sync_governor_target,
                update_governor_gate,
            )
                .chain()
                .before(LightQualitySet),),
        );
    }
}
