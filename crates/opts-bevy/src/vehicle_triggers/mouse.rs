//! Hot-spot triggers: a left click on a `[mouseevent]` mesh raises its trigger, the drag while
//! held and the release.

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use crate::vehicle_sim::PlayerVehicle;

use super::*;

/// The hot-spot held with the left mouse button.
#[derive(Debug, Clone)]
pub struct HeldHotSpot {
    owner: Entity,
    trigger: String,
    dragged: bool,
}

/// Parameters for [`script_mouse_trigger_system`].
#[derive(bevy::ecs::system::SystemParam)]
pub struct ScriptMouseTriggerParams<'w, 's> {
    pub mouse: Res<'w, ButtonInput<MouseButton>>,
    pub motion: MessageReader<'w, 's, MouseMotion>,
    pub vehicles: Query<'w, 's, &'static VehicleScripts, Scripted>,
    pub player: Query<'w, 's, &'static crate::vehicle_sim::VehicleController, With<PlayerVehicle>>,
    pub picker: HotSpotPicker<'w, 's>,
    pub driving: Option<Res<'w, crate::mouse_driving::MouseDriving>>,
}

/// Raises hot-spot triggers for left clicks on [`MouseTrigger`] meshes of the player's vehicle.
pub fn script_mouse_trigger_system(
    mut params: ScriptMouseTriggerParams,
    mut held: Local<Option<HeldHotSpot>>,
) {
    let delta: Vec2 = params.motion.read().map(|m| m.delta).sum();
    let speed = params
        .player
        .iter()
        .next()
        .map_or(0.0, |c| c.forward_speed_km_h());
    if let Some(h) = held.as_mut() {
        if let Ok(scripts) = params.vehicles.get(h.owner) {
            if let Ok(mut engine) = scripts.engine.lock() {
                engine.set_system("mouse_x", delta.x);
                engine.set_system("mouse_y", delta.y);
            }
        }
        // `<name>_drag` is raised on every frame in which the mouse moves while the button is
        // held (the scripts add `mouse_x`/`mouse_y` to a position each time: the Citybus 530's
        // ignition key turns by `mouse_y / -75` per call, so a single call would barely move it).
        if delta != Vec2::ZERO {
            h.dragged = true;
            if !door_lock::held_back(&h.trigger, speed) {
                raise(&params.vehicles, h.owner, &format!("{}_drag", h.trigger));
            }
        }
        if !params.mouse.pressed(MouseButton::Left) {
            if let Ok(scripts) = params.vehicles.get(h.owner) {
                if let Ok(mut engine) = scripts.engine.lock() {
                    engine.set_system("mouse_x", 0.0);
                    engine.set_system("mouse_y", 0.0);
                }
            }
            raise(&params.vehicles, h.owner, &format!("{}_off", h.trigger));
            *held = None;
        }
        return;
    }
    if !params.mouse.just_pressed(MouseButton::Left) {
        return;
    }
    // The cursor is the steering wheel while the mouse drives: it clicks nothing in the cockpit.
    if params.driving.is_some_and(|d| d.drives()) {
        return;
    }
    let Some(spot) = params.picker.under_cursor() else {
        return;
    };
    if door_lock::held_back(&spot.trigger, speed) {
        tracing::debug!(trigger = %spot.trigger, speed, "door hot-spot held back: the vehicle moves");
        return;
    }
    raise(&params.vehicles, spot.owner, &spot.trigger);
    *held = Some(HeldHotSpot {
        owner: spot.owner,
        trigger: spot.trigger.clone(),
        dragged: false,
    });
}
