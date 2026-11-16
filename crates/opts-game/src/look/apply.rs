//! Applying a look: the render options at start, the look on the main cameras, and the plugin
//! that registers the active look.

use bevy::prelude::*;

use super::grading::grading_for;
use super::{ActiveLook, Look};

/// Applies the render options at start: the stored settings, overridden for this run by
/// `--texture-filter` / `--look` (the overrides are not persisted).
pub fn apply_startup_render_options(
    app: &mut App,
    settings: &crate::settings::GameSettings,
    args: &opts_cli::CliArgs,
) {
    let filtering = match args.texture_filter.as_deref() {
        Some(text) => opts_bevy::TextureFiltering::parse(text).unwrap_or_else(|| {
            tracing::warn!(
                value = text,
                "unknown --texture-filter (off, 4, 8, 16); using the setting"
            );
            settings.texture_filtering
        }),
        None => settings.texture_filtering,
    };
    opts_bevy::set_texture_filtering(filtering);
    let look = match args.look.as_deref() {
        Some(text) => Look::parse(text).unwrap_or_else(|| {
            tracing::warn!(
                value = text,
                "unknown --look (vivid, natural, bevy); using the setting"
            );
            settings.look
        }),
        None => settings.look,
    };
    tracing::info!(
        texture_filtering = filtering.key(),
        look = look.key(),
        "render options"
    );
    app.insert_resource(ActiveLook(look));
}

/// Applies the active look to the `MainCamera`: all of them when the look changed, new ones when
/// they appear.
pub fn apply_look_to_cameras(
    look: Res<ActiveLook>,
    cameras: Query<(Entity, Has<LookApplied>), With<opts_bevy::MainCamera>>,
    mut commands: Commands,
) {
    let params = look.0.params();
    let changed = look.is_changed();
    for (entity, applied) in &cameras {
        if changed || !applied {
            commands
                .entity(entity)
                .insert((params.tonemapping, grading_for(params), LookApplied));
        }
    }
}

/// Marks a camera that already carries the active look.
#[derive(Component)]
pub struct LookApplied;

/// Registers [`ActiveLook`] and the system that applies it.
pub struct LookPlugin;

impl Plugin for LookPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveLook>()
            .add_systems(Update, apply_look_to_cameras);
    }
}
