//! Systems of the switch: the toggle on activation, the visual sync and the plugin.

use bevy::prelude::*;

use crate::interaction::{activation::Activated, events::SwitchToggled, state::Disabled};
use crate::tokens::components::BackgroundRole;

use super::{M3Switch, SwitchBuilder, SwitchThumb, SwitchTrack};

/// System toggling a switch when it is [`Activated`].
pub fn switch_toggle_system(
    mut activated: MessageReader<Activated>,
    mut switches: Query<&mut M3Switch, Without<Disabled>>,
    mut events: MessageWriter<SwitchToggled>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(mut switch) = switches.get_mut(*entity) {
            switch.is_on = !switch.is_on;
            events.write(SwitchToggled {
                entity: *entity,
                is_on: switch.is_on,
            });
        }
    }
}

/// System synchronizing switch visual track and thumb nodes when switch state changes.
pub fn switch_sync_system(
    changed_query: Query<(Entity, &M3Switch), Changed<M3Switch>>,
    mut track_query: Query<(&SwitchTrack, &mut Node, &mut BackgroundRole), Without<SwitchThumb>>,
    mut thumb_query: Query<(&SwitchThumb, &mut Node, &mut BackgroundRole), Without<SwitchTrack>>,
) {
    for (entity, switch) in &changed_query {
        let (track_bg, _, justify) = SwitchBuilder::track_styles(switch.is_on);
        let (thumb_bg, thumb_size) = SwitchBuilder::thumb_styles(switch.is_on);

        for (track, mut style, mut bg_role) in &mut track_query {
            if track.0 == entity {
                bg_role.0 = track_bg;
                style.justify_content = justify;
            }
        }

        for (thumb, mut style, mut bg_role) in &mut thumb_query {
            if thumb.0 == entity {
                bg_role.0 = thumb_bg;
                style.width = Val::Px(thumb_size);
                style.height = Val::Px(thumb_size);
            }
        }
    }
}

/// Plugin registering Switch widget systems and events.
#[derive(Default)]
pub struct SwitchPlugin;

impl Plugin for SwitchPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<Messages<SwitchToggled>>() {
            app.add_message::<SwitchToggled>();
        }
        app.add_systems(Update, (switch_toggle_system, switch_sync_system));
    }
}
