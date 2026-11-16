//! Input releases that fall due, and the commands taken from the inbox for this frame.

use bevy::ecs::world::World;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseButtonInput;
use bevy::input::ButtonState;
use bevy::time::{Real, Time};

use super::super::handlers::input::primary_window_or_placeholder;
use super::super::state::{ControlInbox, ControlState, Pending};
use super::COMMANDS_PER_FRAME;

pub(super) fn release_due_inputs(world: &mut World) {
    let now = world.resource::<Time<Real>>().elapsed_secs_f64();
    let due: Vec<_> = {
        let mut state = world.resource_mut::<ControlState>();
        let (due, keep) = std::mem::take(&mut state.releases)
            .into_iter()
            .partition(|r| r.at <= now);
        state.releases = keep;
        due
    };
    let window = primary_window_or_placeholder(world);
    for release in due {
        if let Some((key_code, logical_key)) = release.key {
            world.write_message(KeyboardInput {
                key_code,
                logical_key,
                state: ButtonState::Released,
                text: None,
                repeat: false,
                window,
            });
        }
        if let Some(button) = release.mouse {
            world.write_message(MouseButtonInput {
                button,
                state: ButtonState::Released,
                window,
            });
        }
    }
}

pub(super) fn take_pending(world: &World) -> Vec<Pending> {
    let inbox = world.resource::<ControlInbox>();
    let Ok(rx) = inbox.0.lock() else {
        return Vec::new();
    };
    std::iter::from_fn(|| rx.try_recv().ok())
        .take(COMMANDS_PER_FRAME)
        .collect()
}
