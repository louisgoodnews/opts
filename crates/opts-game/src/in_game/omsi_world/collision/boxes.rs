//! Oriented boxes of the player's parts and of an AI vehicle, in the OMSI frame.

use bevy::prelude::*;
use opts_sim::collision::Obb;

use super::body::{BodyBox, PlayerBox};

/// An OMSI-frame box for a Bevy transform and a local box.
pub(super) fn obb_of(tf: &GlobalTransform, centre: Vec3, half: Vec3) -> Obb {
    let c = tf.transform_point(centre);
    let yaw = tf.compute_transform().rotation.to_euler(EulerRot::YXZ).0;
    let base = tf.translation().y;
    Obb::new(
        Vec2::new(c.x, -c.z),
        Vec2::new(half.x, half.z),
        -yaw,
        base + centre.y - half.y,
        base + centre.y + half.y,
    )
}

/// The player's boxes: head and, for a chain, every following part.
pub(super) fn player_boxes(
    position: Vec2,
    heading: f32,
    z: f32,
    head: &PlayerBox,
    followers: &[(BodyBox, Vec2, f32)],
) -> Vec<Obb> {
    let (s, c) = heading.sin_cos();
    let (fwd, right) = (Vec2::new(s, c), Vec2::new(c, -s));
    let mut out = vec![Obb::new(
        position + right * head.0.offset.x + fwd * head.0.offset.y,
        head.0.half,
        heading,
        z,
        z + head.0.height,
    )];
    for (b, pos, h) in followers {
        let (s, c) = h.sin_cos();
        out.push(Obb::new(
            *pos + Vec2::new(c, -s) * b.offset.x + Vec2::new(s, c) * b.offset.y,
            b.half,
            *h,
            z,
            z + b.height,
        ));
    }
    out
}
