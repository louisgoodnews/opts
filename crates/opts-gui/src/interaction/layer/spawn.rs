//! Spawning of the state layer and scrim helper nodes of every new control.

use bevy::prelude::*;

use super::{ScrimNode, StateLayerLink, StateLayerNode, StateLayered};

fn overlay_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(0.0),
        right: Val::Px(0.0),
        top: Val::Px(0.0),
        bottom: Val::Px(0.0),
        ..default()
    }
}

/// Gives every new control its state layer and scrim.
pub fn spawn_state_layers(
    mut commands: Commands,
    controls: Query<Entity, (Added<StateLayered>, Without<StateLayerLink>)>,
) {
    for control in &controls {
        let layer = commands
            .spawn((
                overlay_node(),
                BackgroundColor(Color::NONE),
                Pickable::IGNORE,
                StateLayerNode,
                Name::new("StateLayer"),
            ))
            .id();
        let scrim = commands
            .spawn((
                overlay_node(),
                BackgroundColor(Color::NONE),
                Pickable::IGNORE,
                ZIndex(1),
                ScrimNode,
                Name::new("DisabledScrim"),
            ))
            .id();
        let link = StateLayerLink { layer, scrim };
        commands
            .entity(control)
            .queue_silenced(move |mut entity: EntityWorldMut| {
                entity.insert(link);
                entity.insert_children(0, &[layer]);
                entity.add_child(scrim);
            });
    }
}
