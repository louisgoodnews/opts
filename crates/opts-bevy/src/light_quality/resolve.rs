//! Resolving the light quality from its inputs, and the plugin that keeps it up to date.

use super::*;

/// What the game asks for: the mode and, for [`LightMode::Custom`], the user's values.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LightQualityInput {
    /// The requested mode.
    pub mode: LightMode,
    /// The user's custom values (their `mode` and `level` fields are ignored).
    pub custom: LightQuality,
}

/// Makes [`LightQuality`] follow the mode, the custom values and the governor's level.
pub fn resolve_light_quality(
    input: Res<LightQualityInput>,
    state: Res<GovernorState>,
    mut quality: ResMut<LightQuality>,
) {
    let want = match input.mode {
        LightMode::Automatic => ladder_preset(state.level),
        LightMode::Custom => input
            .custom
            .clamped()
            .with_mode(LightMode::Custom, state.level),
    };
    if *quality != want {
        *quality = want;
    }
}

/// Registers the lighting quality state and the governor.
#[derive(Default)]
pub struct LightQualityPlugin;

impl Plugin for LightQualityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightQuality>()
            .init_resource::<LightQualityInput>()
            .init_resource::<GovernorSettings>()
            .init_resource::<GovernorState>()
            .init_resource::<GovernorGate>()
            .init_resource::<GovernorTargetFps>()
            .init_resource::<GovernorBudget>()
            .add_systems(
                Update,
                (update_governor_budget, run_governor, resolve_light_quality)
                    .chain()
                    .in_set(LightQualitySet),
            );
    }
}

/// The governor and the resolution of [`LightQuality`]; schedule input writers `.before` it and
/// consumers `.after` it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LightQualitySet;
