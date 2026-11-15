//! Gates of the drive controls: the modal, the player, the vehicle and its gear.


use super::*;


pub(super) fn resolve_trigger(bindings: &ScriptKeyBindings, name: &str) -> String {
    if bindings.0.iter().any(|b| b.trigger == name) {
        return name.to_string();
    }
    TRIGGER_ALIASES
        .iter()
        .find(|(alias, _)| *alias == name)
        .map_or_else(|| name.to_string(), |(_, real)| (*real).to_string())
}

// ---- gates ----------------------------------------------------------------------------------

pub(super) fn modal_open(world: &World) -> bool {
    world
        .get_resource::<opts_bevy::reporting::state::ModalActive>()
        .is_some_and(|m| m.0)
        || world
            .get_resource::<opts_bevy::reporting::state::MenuModalActive>()
            .is_some_and(|m| m.0)
}

pub(super) fn has_player(world: &mut World) -> bool {
    world
        .query_filtered::<(), With<PlayerVehicle>>()
        .iter(world)
        .next()
        .is_some()
}

/// The conditions under which the vehicle input systems run, checked now.
pub(super) fn vehicle_gate(world: &mut World) -> Result<(), RpcError> {
    if !has_player(world) {
        return Err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (no drive is running)",
        ));
    }
    if modal_open(world) {
        return Err(RpcError::new(
            code::GATED,
            "a dialog or the vehicle menu is open: vehicle input is blocked, exactly like a key press",
        ));
    }
    if world
        .get_resource::<CameraState>()
        .is_some_and(|c| c.mode == CameraMode::FreeRoam)
    {
        return Err(RpcError::new(
            code::GATED,
            "the camera is in free roam: the vehicle takes no input (`camera.set_mode external` first)",
        ));
    }
    Ok(())
}

pub(super) fn trigger_gate(world: &mut World) -> Result<(), RpcError> {
    if !has_player(world) {
        return Err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (no drive is running)",
        ));
    }
    if modal_open(world) {
        return Err(RpcError::new(
            code::GATED,
            "a dialog or the vehicle menu is open: vehicle triggers are blocked, exactly like a key press",
        ));
    }
    Ok(())
}

pub(super) fn gear_name(g: GearPosition) -> &'static str {
    match g {
        GearPosition::Drive => "drive",
        GearPosition::Neutral => "neutral",
        GearPosition::Reverse => "reverse",
        #[allow(unreachable_patterns)]
        _ => "other",
    }
}

// ---- drive.controls ---------------------------------------------------------------------------
