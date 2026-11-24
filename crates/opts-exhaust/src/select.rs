//! The emitters of a frame: the vehicles that put out puffs (the player's, and the nearest others
//! with an outlet), each with its outlet and its faded rate.

use bevy::prelude::*;
use opts_bevy::vehicle::ExhaustOutlet;
use opts_bevy::{ArticulatedChain, ArticulatedSegment, PlayerVehicle, VehicleController};
use opts_sim::exhaust::puff_rate_per_s;

use crate::nearest::{fade_of, nearest, AI_LOAD, NEAREST_N};
use crate::outlet::chain_outlet;

/// A vehicle that can put out puffs: the player's, or one with an outlet. Trailer segments are not
/// vehicles of their own (their consist's root emits for them).
pub(crate) type Vehicles<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GlobalTransform,
        Option<&'static ExhaustOutlet>,
        Option<&'static VehicleController>,
        Has<PlayerVehicle>,
    ),
    (
        Or<(With<PlayerVehicle>, With<ExhaustOutlet>)>,
        Without<ArticulatedSegment>,
    ),
>;

/// One emitter of this frame: the vehicle, its outlet and its puff rate (per second, faded).
pub(crate) struct Emitter {
    pub(crate) entity: Entity,
    pub(crate) outlet: Vec3,
    pub(crate) rate: f32,
}

/// The player's vehicle (always) and the [`NEAREST_N`] nearest other vehicles with an outlet,
/// each with its outlet and its faded rate. The player comes first, so it keeps its puffs when the
/// cap is short.
pub(crate) fn emitters(
    eye: Vec3,
    temp_c: f32,
    vehicles: &Vehicles,
    chains: &Query<&ArticulatedChain>,
    units: &Query<(&GlobalTransform, Option<&ExhaustOutlet>)>,
) -> Vec<Emitter> {
    let mut player = None;
    let mut others = Vec::new();
    for (entity, tf, outlet, _, is_player) in vehicles.iter() {
        let distance = eye.distance(tf.translation());
        if is_player {
            player = Some((entity, distance));
        } else if outlet.is_some() {
            others.push((entity, distance));
        }
    }
    nearest(player, others, NEAREST_N)
        .into_iter()
        .filter_map(|(entity, distance)| {
            let (_, tf, data, controller, _) = vehicles.get(entity).ok()?;
            let load = controller.map_or(AI_LOAD, |c| c.inputs().throttle());
            let trailers = chains
                .get(entity)
                .ok()
                .into_iter()
                .flat_map(|c| c.segments.iter())
                .filter_map(|&s| units.get(s).ok());
            Some(Emitter {
                entity,
                outlet: chain_outlet(tf, data, trailers),
                rate: puff_rate_per_s(temp_c, load) * fade_of(distance),
            })
        })
        .collect()
}
