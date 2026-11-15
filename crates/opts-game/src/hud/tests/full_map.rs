//! The full-size map (M-0108): opening, following, closing, the redraw policy and the input hold.

use super::map_widget::{map_app, run};
use super::*;
use crate::hud::fullmap::{FullMapRequest, FullMapRoot, FullMapState};

fn full(app: &App) -> FullMapState {
    app.world().resource::<FullMapState>().clone()
}

fn press(app: &mut App, keys: &[KeyCode]) {
    use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
    use bevy::input::ButtonState;
    for state in [ButtonState::Pressed, ButtonState::Released] {
        for k in keys {
            app.world_mut().write_message(KeyboardInput {
                key_code: *k,
                logical_key: Key::Unidentified(NativeKey::Unidentified),
                state,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        }
        for _ in 0..2 {
            app.update();
        }
    }
}

fn open_map(app: &mut App) {
    app.world_mut().resource_mut::<FullMapState>().request = Some(FullMapRequest::Open);
    run(app, 3);
}

fn roots(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<(), With<FullMapRoot>>()
        .iter(app.world())
        .count()
}
mod follow;
mod open;
mod rpc;

