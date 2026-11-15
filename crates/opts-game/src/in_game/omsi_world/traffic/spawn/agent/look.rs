//! What a spawned vehicle needs from its variant: sounds, door reach, wheel and axle geometry,
//! names and the chain of trailing units.

use std::sync::Arc;

use bevy::prelude::*;
use opts_traffic::{IdmParams, VehicleClass};

use super::*;

/// What the spawned vehicle needs from its variant: sounds, the front door's reach, wheel and
/// axle geometry, the names and the chain of trailing units.
pub(super) struct VariantLook {
    pub(super) sound: Option<SoundSet>,
    pub(super) door_ahead: f32,
    pub(super) unit_sounds: Vec<Option<SoundSet>>,
    pub(super) wheel_radius: f32,
    pub(super) rear_axle_y: f32,
    pub(super) delta_height: f32,
    pub(super) name: String,
    pub(super) measured: bool,
    pub(super) head_bus_path: std::path::PathBuf,
    pub(super) chain: Vec<ChainUnit>,
}

pub(super) fn variant_look(
    variant: &crate::in_game::omsi_world::traffic_variant::Variant,
    params: &IdmParams,
    entities: &[Entity],
) -> VariantLook {
    let head = &variant.units[0];
    let sound = head.sound.clone();
    let door_ahead = head
        .front_door_y
        .map_or(0.0, |y| (head.size.unwrap_or((4.4, 2.2)).1 - y).max(0.0));
    let unit_sounds: Vec<Option<Arc<crate::vehicle_sound::AiSoundSet>>> = variant
        .units
        .iter()
        .skip(1)
        .map(|u| u.sound.clone())
        .collect();
    let size = head.size.unwrap_or((4.4, 2.2));
    let heavy = params.class != VehicleClass::Car;
    let wheel_radius = head
        .profile
        .wheel_radius_m
        .unwrap_or(if heavy { 0.5 } else { 0.31 });
    let rear_axle_y = size.1 - (params.front_overhang + params.wheelbase) as f32;
    let delta_height = head.profile.delta_height_m;
    let name = variant.name.clone();
    let measured = head.response.is_some();
    let head_bus_path = head.bus_path.clone();
    let chain: Vec<ChainUnit> = variant
        .units
        .iter()
        .zip(entities)
        .skip(1)
        .map(|(u, &e)| {
            ChainUnit::new(
                e,
                u.hitch,
                u.joint,
                u.rear_axle_y,
                u.max_alpha_deg,
                u.reversed,
            )
        })
        .collect();
    VariantLook {
        sound,
        door_ahead,
        unit_sounds,
        wheel_radius,
        rear_axle_y,
        delta_height,
        name,
        measured,
        head_bus_path,
        chain,
    }
}
