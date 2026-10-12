//! Bevy plugin registering all main menu systems, states, and event listeners.

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::menu::{
    background::sync_menu_scrim,
    entries::quit_button_action_system,
    interactions::{
        menu_focus_visual_system, menu_interaction_state_system, menu_keyboard_activation_system,
    },
    systems::{
        enter_main_menu_system, exit_main_menu_system, initial_focus_system,
        menu_escape_exit_system,
    },
    theme_toggle::{sync_theme_toggle_glyph, theme_toggle_system},
};

/// Plugin managing the lifecycle, rendering, and interaction systems for the OPTS Main Menu.
pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        // Lifecycle triggers
        app.add_systems(OnEnter(AppState::MainMenu), enter_main_menu_system)
            .add_systems(OnExit(AppState::MainMenu), exit_main_menu_system);

        // Active menu systems
        app.add_systems(
            Update,
            (
                menu_focus_visual_system,
                menu_keyboard_activation_system,
                menu_interaction_state_system,
                theme_toggle_system,
                sync_theme_toggle_glyph,
                sync_menu_scrim,
                initial_focus_system,
                menu_escape_exit_system,
                quit_button_action_system,
            )
                .run_if(in_state(AppState::MainMenu))
                .run_if(opts_bevy::reporting::not_modal_active),
        );
    }
}
