//! Game-specific methods of the control protocol: `state`, `scene`, `scene.load_map`,
//! `scene.unload`, `vehicle.set`, `camera.teleport`, `render.look`. The generic ones live in
//! `opts_bevy::control`; see `docs/src/architecture/control-protocol.md`.

use bevy::prelude::*;
use opts_bevy::control::state::ControlRegistry;

mod context;
mod handlers;
mod render;
mod restart;
mod status;
mod teleport;

pub(crate) use restart::restart_waiter;
use context::{allow_session_reads, capture_metadata};
use handlers::{
    menu_do, menu_state, script_vars, ui_navigate, vehicle_get,
};
use render::{render_look, render_stats};
use restart::set_vehicle;
use status::{scene, state, unload};
use teleport::teleport;

/// Registers the game's control methods (needs `opts_bevy::control::ControlPlugin` first).
pub struct GameControlPlugin;

impl Plugin for GameControlPlugin {
    fn build(&self, app: &mut App) {
        let Some(mut registry) = app.world_mut().get_resource_mut::<ControlRegistry>() else {
            return;
        };
        registry.register("state", state);
        registry.register("scene", scene);
        registry.register("scene.load_map", crate::control_loading::load_map);
        registry.register("scene.unload", unload);
        registry.register("vehicle.set", set_vehicle);
        registry.register("vehicle.script_vars", script_vars);
        registry.register("vehicle.get", vehicle_get);
        registry.register("camera.teleport", teleport);
        registry.register("render.look", render_look);
        registry.register("render.stats", render_stats);
        registry.register("ui.navigate", ui_navigate);
        registry.register("menu.do", menu_do);
        registry.register("menu.state", menu_state);
        crate::hud::protocol::register(&mut registry);
        crate::passengers::register(&mut registry);
        crate::language_packs::register(&mut registry);
        crate::control_ui::register(&mut registry);
        crate::control_drive::register(&mut registry);
        crate::viewpoints::protocol::register(&mut registry);
        crate::control_session::register(&mut registry);
        crate::control_environment::register(&mut registry);
        crate::formation::register(&mut registry);
        crate::control_save::register(&mut registry);
        crate::control_trace::register(&mut registry);
        crate::control_tiles::register(&mut registry);
        crate::control_audio::register(&mut registry);
        crate::control_hotspots::register(&mut registry);
        crate::control_textures::register(&mut registry);
        crate::control_loading::register(&mut registry);
        crate::control_setvars::register(&mut registry);
        crate::control_configs::register(&mut registry);
        crate::control_dynamics::register(&mut registry);
        crate::control_route::register(&mut registry);
        crate::control_scheduled::register(&mut registry);
        crate::control_rail::register(&mut registry);
        crate::control_profiles::register(&mut registry);
        crate::control_trace::add_overlay(app);
        app.add_systems(Update, allow_session_reads);
        if let Some(mut hooks) = app
            .world_mut()
            .get_resource_mut::<opts_bevy::capture::CaptureMetaHooks>()
        {
            hooks.0.push(capture_metadata);
        }
    }
}

#[cfg(test)]
mod restart_waiter_tests;
#[cfg(test)]
mod tests;
