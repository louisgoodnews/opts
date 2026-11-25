//! Trailing units of coupled AI vehicles (articulated buses, lorries with trailers): each unit
//! follows the unit ahead through the same kinematic joint the player's consist uses
//! ([`TrailerKinematics`]), pulled behind the lead vehicle that the lane model drives.

use bevy::math::{DVec2, Vec2};
use bevy::prelude::*;
use opts_bevy::coords::{omsi_euler_to_bevy_quat, omsi_to_bevy_pos};
use opts_sim::vehicle::trailer::TrailerKinematics;

use super::traffic::TrafficVehicle;

/// One trailing unit.
pub struct ChainUnit {
    entity: Entity,
    hitch: DVec2,
    joint: DVec2,
    axle_distance: f64,
    max_alpha_deg: f64,
    reversed: bool,
    kin: Option<TrailerKinematics>,
}

impl ChainUnit {
    /// A unit whose front `joint` sits on its predecessor's rear coupling `hitch` (both in
    /// their own vehicle frames), with its rearmost axle at `rear_axle_y`.
    #[must_use]
    pub fn new(
        entity: Entity,
        hitch: Vec2,
        joint: Vec2,
        rear_axle_y: f32,
        max_alpha_deg: f32,
        reversed: bool,
    ) -> Self {
        Self {
            entity,
            hitch: hitch.as_dvec2(),
            joint: joint.as_dvec2(),
            axle_distance: f64::from((joint.y - rear_axle_y).max(1.0)),
            max_alpha_deg: f64::from(max_alpha_deg),
            reversed,
            kin: None,
        }
    }
}

/// The trailing units of a lead vehicle, in order.
#[derive(Component)]
pub struct TrafficChain {
    pub(super) units: Vec<ChainUnit>,
}

/// Despawns a lead vehicle together with its trailing units.
pub(super) fn despawn_chain(commands: &mut Commands, lead: Entity, chain: Option<&TrafficChain>) {
    for unit in chain.into_iter().flat_map(|c| &c.units) {
        commands.entity(unit.entity).despawn();
    }
    commands.entity(lead).despawn();
}

/// Places the trailing units behind their lead vehicle every frame.
pub fn move_traffic_chains(
    time: Res<Time>,
    mut leads: Query<(&TrafficVehicle, &mut TrafficChain)>,
    mut parts: Query<(&mut Transform, &mut Visibility), Without<TrafficVehicle>>,
) {
    let dt = f64::from(time.delta_secs().min(0.2));
    for (lead, mut chain) in &mut leads {
        let mut leader = (lead.origin, lead.heading);
        for unit in &mut chain.units {
            let (hitch, joint, axle, alpha) = (
                unit.hitch,
                unit.joint,
                unit.axle_distance,
                unit.max_alpha_deg,
            );
            let kin = unit.kin.get_or_insert_with(|| {
                TrailerKinematics::new(hitch, joint, axle, alpha, leader.0, leader.1)
            });
            let state = kin.step(leader.0, leader.1, dt);
            if let Ok((mut transform, mut visibility)) = parts.get_mut(unit.entity) {
                transform.translation = omsi_to_bevy_pos(Vec3::new(
                    state.origin.x as f32,
                    state.origin.y as f32,
                    lead.height,
                ));
                let turn = if unit.reversed { 180.0 } else { 0.0 };
                transform.rotation =
                    omsi_euler_to_bevy_quat(-(state.heading.to_degrees() as f32 + turn), 0.0, 0.0);
                if *visibility != Visibility::Inherited {
                    *visibility = Visibility::Inherited;
                }
            }
            leader = (state.origin, state.heading);
        }
    }
}

#[cfg(test)]
mod tests;
