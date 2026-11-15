//! Debug drawing of the traffic lanes and agent ids. F11 toggles the lane centre lines and their
//! successor arrows (`lanes`), Shift+F11 the agent ids (`agent_id_overlay`). Nothing is drawn
//! unless the developer panel is on; the status line under the panel names the layers.

use bevy::prelude::*;

use crate::app_state::AppState;
use crate::in_game::omsi_world::WorldStats;
use crate::in_game::types::InGameEntity;

mod agent_id_overlay;
mod lanes;
#[cfg(test)]
mod tests;

/// Which traffic debug layers are on (F11 and Shift+F11).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LaneOverlay {
    /// Lane centre lines and successor arrows.
    pub lanes: bool,
    /// Agent id markers and labels.
    pub agent_ids: bool,
}

fn on_off(on: bool) -> &'static str {
    if on {
        "on"
    } else {
        "off"
    }
}

/// The status line naming the layers and their keys.
#[must_use]
pub fn layer_status(o: &LaneOverlay) -> String {
    format!(
        "[F11 lanes {} | Shift+F11 agent ids {}]",
        on_off(o.lanes),
        on_off(o.agent_ids)
    )
}

/// F11 switches the lanes, Shift+F11 the agent ids.
pub fn toggle_lane_overlay(keyboard: Res<ButtonInput<KeyCode>>, mut overlay: ResMut<LaneOverlay>) {
    if !keyboard.just_pressed(KeyCode::F11) {
        return;
    }
    if keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        overlay.agent_ids = !overlay.agent_ids;
    } else {
        overlay.lanes = !overlay.lanes;
    }
}

/// Marks the status line of the layers (spawned with the first frame of the overlay).
#[derive(Component)]
pub struct LayerStatusText;

/// Keeps the layer status line under the developer panel up to date.
pub fn update_layer_status(
    mut commands: Commands,
    stats: Res<WorldStats>,
    overlay: Res<LaneOverlay>,
    mut texts: Query<(&mut Text, &mut Visibility), With<LayerStatusText>>,
) {
    let line = layer_status(&overlay);
    if texts.is_empty() {
        commands.spawn((
            Text::new(line),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(Color::srgb(0.6, 1.0, 0.8)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(70.0),
                left: Val::Px(20.0),
                ..default()
            },
            Visibility::Hidden,
            LayerStatusText,
            InGameEntity,
        ));
        return;
    }
    let vis = if stats.overlay_visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for (mut text, mut v) in &mut texts {
        if text.0 != line {
            text.0.clone_from(&line);
        }
        if *v != vis {
            *v = vis;
        }
    }
}

/// Registers the lane overlay systems (called from the in-game plugin).
pub(crate) fn add_lane_overlay(app: &mut App) {
    app.init_resource::<LaneOverlay>().add_systems(
        Update,
        (
            toggle_lane_overlay,
            lanes::draw_lanes,
            update_layer_status,
            agent_id_overlay::update_agent_ids,
        )
            .run_if(in_state(AppState::InGame))
            .run_if(opts_bevy::reporting::not_modal_active),
    );
}
