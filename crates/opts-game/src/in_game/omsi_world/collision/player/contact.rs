//! The contact search of the collision pass: the deepest overlap of the player's boxes with the
//! static boxes and the AI vehicles, and the steps in the ground around a footprint.

use bevy::prelude::*;
use opts_sim::collision::Obb;

use super::super::super::ground_height::GroundIndex;
use super::super::CollisionIndex;
use super::STEP_M;

/// The deepest overlap of the player's boxes with the static boxes and the AI vehicles. Static
/// boxes the vehicle was placed inside (a spawn point in a bus stop's collision box) do not push
/// it: it is on its way out when it drives off; `touching` collects the boxes it still touches.
#[derive(Default)]
pub(super) struct Worst {
    pub(super) contact: Option<(Vec2, f32)>,
    pub(super) traffic: bool,
    pub(super) other: Option<Obb>,
}

pub(super) fn worst_contact(
    mine: &[Obb],
    (index, dynamic): (&CollisionIndex, &[Obb]),
    (first_seen, free, touching): (bool, &mut Vec<Obb>, &mut Vec<Obb>),
) -> Worst {
    let mut worst = Worst::default();
    for me in mine {
        let statics = index.grid.query(me.centre, me.radius()).map(|o| (o, false));
        for (other, is_traffic) in statics.chain(dynamic.iter().map(|o| (o, true))) {
            if !me.overlaps_height(other, STEP_M) {
                continue;
            }
            if let Some(c) = me.contact(other) {
                if !is_traffic {
                    if first_seen && !free.contains(other) {
                        free.push(*other);
                    }
                    if free.contains(other) {
                        if !touching.contains(other) {
                            touching.push(*other);
                        }
                        continue;
                    }
                }
                if worst.contact.is_none_or(|w| c.depth > w.1) {
                    worst.contact = Some((c.normal, c.depth));
                    worst.traffic = is_traffic;
                    worst.other = Some(*other);
                }
            }
        }
    }
    worst
}

/// Direction away from a step in the ground around a footprint, if there is one: points on the
/// outline are compared with the ground 0.4 m closer to the centre. A ground higher than
/// `clearance` above the vehicle (the deck of a bridge over the road) is no step.
pub(super) fn ground_step(
    ground: &GroundIndex,
    body: &Obb,
    z: f32,
    clearance: f32,
) -> Option<Vec2> {
    let [fwd, right] = body.axes();
    let mut away = Vec2::ZERO;
    for (a, b) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        for k in [-0.8_f32, 0.0, 0.8] {
            let (lx, ly) = if a != 0.0 {
                (a * body.half.x, k * body.half.y)
            } else {
                (k * body.half.x, b * body.half.y)
            };
            let edge = body.centre + right * lx + fwd * ly;
            let inward = (body.centre - edge).normalize_or_zero();
            let (Some(outer), Some(inner)) = (
                ground.height_at(edge.x + (-inward.x) * 0.1, edge.y + (-inward.y) * 0.1, z),
                ground.height_at(edge.x + inward.x * 0.3, edge.y + inward.y * 0.3, z),
            ) else {
                continue;
            };
            // A surface higher than the vehicle is a deck it passes under, not a step.
            if outer - inner > STEP_M && outer > z + STEP_M && outer < z + clearance {
                away += inward;
            }
        }
    }
    (away.length_squared() > 0.0).then(|| away.normalize())
}
