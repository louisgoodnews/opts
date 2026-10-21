//! Clicks made by a command instead of the mouse (the control protocol's `vehicle.click`).

use bevy::prelude::*;

use crate::vehicle_script::VehicleScripts;
use crate::vehicle_sim::PlayerVehicle;

/// A click on a hot-spot made by a command instead of the mouse (the control protocol's
/// `vehicle.click`): press, drag by an offset over some frames, hold, release. Everything goes
/// through the same triggers and system values as a real click, so it tests the controls the way
/// a player uses them.
#[derive(Debug, Clone)]
pub struct ScriptedClick {
    /// The hot-spot's trigger name (`[mouseevent]`), without the `_drag`/`_off` suffix.
    pub trigger: String,
    /// Total mouse movement to apply while dragging, in pixels (`mouse_x`, `mouse_y`).
    pub drag: Vec2,
    /// Frames the drag is spread over (at least 1 when `drag` is not zero).
    pub drag_frames: u32,
    /// Seconds to keep the button held after the drag, before the release.
    pub hold_s: f32,
    /// Frames since the press (0: not pressed yet).
    frame: u32,
    /// Seconds held after the drag.
    held_s: f32,
}

impl ScriptedClick {
    /// A click that presses `trigger`, drags by `drag` pixels over `drag_frames` frames, holds for
    /// `hold_s` seconds and releases.
    #[must_use]
    pub fn new(trigger: &str, drag: Vec2, drag_frames: u32, hold_s: f32) -> Self {
        Self {
            trigger: trigger.to_owned(),
            drag,
            drag_frames: if drag == Vec2::ZERO {
                0
            } else {
                drag_frames.max(1)
            },
            hold_s: hold_s.max(0.0),
            frame: 0,
            held_s: 0.0,
        }
    }
}

/// The clicks waiting to run or running; one frame of each per update.
#[derive(Resource, Default, Debug)]
pub struct ScriptedClicks(pub Vec<ScriptedClick>);

/// Runs the [`ScriptedClicks`] against the player's vehicle.
pub fn scripted_click_system(
    time: Res<Time>,
    mut clicks: ResMut<ScriptedClicks>,
    vehicles: Query<(Entity, &VehicleScripts), With<PlayerVehicle>>,
    consists: Query<&crate::consist_scripts::ScriptConsist>,
    parts: Query<&VehicleScripts, With<crate::consist_scripts::ScriptPart>>,
) {
    if clicks.0.is_empty() {
        return;
    }
    let Some((head, head_scripts)) = vehicles.iter().next() else {
        clicks.0.clear();
        return;
    };
    let mut done = Vec::new();
    for (i, c) in clicks.0.iter_mut().enumerate() {
        // The unit that owns the hot-spot's trigger: the head, else the first part that has it.
        let owner = std::iter::once(head_scripts)
            .chain(
                consists
                    .get(head)
                    .iter()
                    .flat_map(|c| &c.units)
                    .filter_map(|u| u.part.and_then(|e| parts.get(e).ok())),
            )
            .find(|s| s.engine.lock().is_ok_and(|e| e.has_trigger(&c.trigger)))
            .unwrap_or(head_scripts);
        let Ok(mut engine) = owner.engine.lock() else {
            continue;
        };
        if c.frame == 0 {
            let handled = engine.trigger(&c.trigger);
            tracing::debug!(trigger = %c.trigger, handled, "vehicle script trigger (scripted click)");
        } else if c.frame <= c.drag_frames {
            let step = c.drag / c.drag_frames as f32;
            engine.set_system("mouse_x", step.x);
            engine.set_system("mouse_y", step.y);
            engine.trigger(&format!("{}_drag", c.trigger));
        } else {
            engine.set_system("mouse_x", 0.0);
            engine.set_system("mouse_y", 0.0);
            c.held_s += time.delta_secs();
            if c.held_s >= c.hold_s {
                engine.trigger(&format!("{}_off", c.trigger));
                done.push(i);
            }
        }
        c.frame += 1;
    }
    for i in done.into_iter().rev() {
        clicks.0.remove(i);
    }
}
