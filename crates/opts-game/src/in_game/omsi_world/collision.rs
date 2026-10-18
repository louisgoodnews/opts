//! Collisions of the player's vehicle: with the scenery objects' collision meshes, with AI
//! vehicles, and with steps in the ground (walls, embankments, platform edges).
//!
//! * **Objects.** Every placed `.sco` with `[collision_mesh]` registers the bounding box of that
//!   mesh as an oriented box (`opts_sim::collision`) in [`CollisionIndex`] when its tile loads and
//!   leaves with the tile. A box whose top is no higher than a step above the vehicle's wheels
//!   (kerb, road plate) is driven over, one above the vehicle is passed under.
//! * **AI vehicles.** The box of an AI vehicle is measured from its meshes once and follows it;
//!   it is solid for the player, which is pushed out (the AI agent itself is kinematic).
//! * **Ground steps.** Points around the footprint are compared with the ground a little closer
//!   to the centre; a rise of more than a kerb over a short distance blocks the vehicle.
//! * **Rail vehicles** are ghosts for others, as in OMSI (Tram SDK): trams neither collide nor
//!   are pushed.
//!
//! The response moves the chassis out of the overlap (at most half a metre per step), removes the
//! velocity into the obstacle and scrapes a little off the rest.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use opts_bevy::{ArticulatedChain, ArticulatedSegment, PlayerVehicle, VehicleController};
use opts_core::formats::scenery::BaseSurface;
use opts_sim::collision::{respond, ColliderGrid, Obb};

use super::ground_height::GroundIndex;
use super::rail::VehicleComposition;
use super::traffic::TrafficVehicle;
use super::{RailPlayer, WorldLoadJob};

/// Highest obstacle a vehicle drives over: kerbs and road plates (m).
const STEP_M: f32 = 0.3;
/// Largest push out of an overlap in one fixed step (m).
const MAX_PUSH_M: f32 = 0.5;
/// Share of the speed along an obstacle that is lost scraping it.
const FRICTION: f32 = 0.08;
/// Bounce off an obstacle.
const RESTITUTION: f32 = 0.05;
/// Cell size of the static index (m).
const CELL_M: f32 = 16.0;

/// The static colliders of the resident tiles, in the session frame (OMSI axes).
#[derive(Resource)]
pub struct CollisionIndex {
    grid: ColliderGrid<(i32, i32)>,
}

impl Default for CollisionIndex {
    fn default() -> Self {
        Self {
            grid: ColliderGrid::new(CELL_M),
        }
    }
}

impl CollisionIndex {
    /// Number of stored entries (for diagnostics).
    #[must_use]
    pub fn len(&self) -> usize {
        self.grid.len()
    }

    /// Whether nothing is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.grid.is_empty()
    }
}

/// The footprint of a vehicle part in its own frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyBox {
    /// Half extents `(right, forward)` (m).
    pub half: Vec2,
    /// Centre offset `(right, forward)` (m).
    pub offset: Vec2,
    /// Height of the body above the ground (m).
    pub height: f32,
}

impl BodyBox {
    /// From a `.bus` `[boundingbox]` (full extents and centre offset, x lateral, y longitudinal).
    #[must_use]
    pub fn from_bounding_box(size: Vec3, centre: Vec3) -> Self {
        Self {
            half: Vec2::new(size.x, size.y) * 0.5,
            offset: Vec2::new(centre.x, centre.y),
            height: size.z.max(1.0),
        }
    }
}

/// The box of the player's head unit (`[boundingbox]` of its `.bus`).
#[derive(Component, Debug, Clone, Copy)]
pub struct PlayerBox(pub BodyBox);

/// The measured box of an AI vehicle, in its own frame: centre and half extents of its meshes
/// `(right, up, back)` as Bevy sees them.
#[derive(Component, Debug, Clone, Copy)]
pub struct VehicleCollider {
    centre: Vec3,
    half: Vec3,
}

impl WorldLoadJob {
    /// Bounding box of a collision mesh in Bevy's local axes, cached; `None` for unreadable
    /// (protected) or missing meshes.
    ///
    /// `bend` is the crossing base the object is bent over (`[crossing_heightdeformation]`).
    pub(super) fn collision_bounds(
        &mut self,
        model_dir: &Path,
        rel: &str,
        bend: Option<&(PathBuf, Arc<BaseSurface>)>,
    ) -> Option<(Vec3, Vec3)> {
        let path = self.install.find_in(model_dir, rel)?;
        let key = (path.clone(), bend.map(|b| b.0.clone()));
        if let Some(b) = self.collision_bounds.get(&key) {
            return *b;
        }
        let bounds = (|| {
            let mut models = opts_bevy::load_decoded_meshes(&path).ok()?;
            if let Some((_, base)) = bend {
                for m in &mut models {
                    base.bend_mesh(m);
                }
            }
            let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for m in &models {
                if matches!(
                    m.vertex_layout(),
                    opts_core::formats::o3d::VertexLayout::Unreadable
                ) {
                    return None;
                }
                let mesh = opts_bevy::o3d_to_bevy_mesh(m);
                if let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                {
                    for p in pos {
                        lo = lo.min(Vec3::from(*p));
                        hi = hi.max(Vec3::from(*p));
                    }
                }
            }
            (lo.x <= hi.x).then_some((lo, hi))
        })();
        self.collision_bounds.insert(key, bounds);
        bounds
    }
}

/// Registers the collision meshes of a placed object (called while its tile is built).
pub(super) fn register_object(
    job: &mut WorldLoadJob,
    commands: &mut Commands,
    sco: &opts_core::formats::scenery::SceneryObject,
    model_dir: &Path,
    tf: &Transform,
    key: (i32, i32),
    bend: Option<&(PathBuf, Arc<BaseSurface>)>,
) {
    if sco.no_collision || sco.collision_meshes.is_empty() {
        return;
    }
    let yaw = tf.rotation.to_euler(EulerRot::YXZ).0;
    let heading = -yaw;
    let mut boxes = Vec::new();
    for rel in &sco.collision_meshes {
        let Some((lo, hi)) = job.collision_bounds(model_dir, rel, bend) else {
            continue;
        };
        let local_centre = (lo + hi) * 0.5;
        let half = (hi - lo) * 0.5;
        let c = tf.transform_point(local_centre);
        boxes.push(Obb::new(
            Vec2::new(c.x, -c.z),
            Vec2::new(half.x, half.z),
            heading,
            tf.translation.y + lo.y,
            tf.translation.y + hi.y,
        ));
    }
    if boxes.is_empty() {
        return;
    }
    commands.queue(move |world: &mut World| {
        if let Some(mut index) = world.get_resource_mut::<CollisionIndex>() {
            for b in boxes {
                index.grid.insert(key, b);
            }
        }
    });
}

/// Drops a tile's colliders.
pub(super) fn unregister_tile(commands: &mut Commands, key: (i32, i32)) {
    commands.queue(move |world: &mut World| {
        if let Some(mut index) = world.get_resource_mut::<CollisionIndex>() {
            index.grid.remove(&key);
        }
    });
}

/// AI vehicles whose box has not been measured yet.
type Unmeasured = (With<TrafficVehicle>, Without<VehicleCollider>);

/// What the player's collision pass reads and moves.
type PlayerCollide = (
    &'static mut VehicleController,
    &'static PlayerBox,
    Option<&'static ArticulatedChain>,
    Option<&'static VehicleComposition>,
);

/// Road vehicles of the player (rail vehicles are ghosts).
type GroundPlayer = (
    Or<(
        With<crate::session_vehicles::SessionVehicle>,
        With<PlayerVehicle>,
    )>,
    Without<RailPlayer>,
);

/// Measures the box of AI vehicles from their meshes once these have bounds.
pub fn attach_traffic_colliders(
    mut commands: Commands,
    vehicles: Query<(Entity, &GlobalTransform), Unmeasured>,
    children: Query<&Children>,
    bounds: Query<(&Aabb, &GlobalTransform)>,
) {
    for (e, root) in &vehicles {
        let inverse = root.affine().inverse();
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        let mut count = 0;
        for d in children.iter_descendants(e) {
            let Ok((aabb, g)) = bounds.get(d) else {
                continue;
            };
            let (c, h) = (Vec3::from(aabb.center), Vec3::from(aabb.half_extents));
            // Shadow volumes and helper geometry are far larger than a vehicle.
            if h.max_element() > 9.0 {
                continue;
            }
            for sx in [-1.0, 1.0] {
                for sy in [-1.0, 1.0] {
                    for sz in [-1.0, 1.0] {
                        let p = g.transform_point(c + h * Vec3::new(sx, sy, sz));
                        let local = inverse.transform_point3(p);
                        lo = lo.min(local);
                        hi = hi.max(local);
                    }
                }
            }
            count += 1;
        }
        if count >= 3 && hi.x > lo.x {
            commands.entity(e).insert(VehicleCollider {
                centre: (lo + hi) * 0.5,
                half: (hi - lo) * 0.5,
            });
        }
    }
}

/// An OMSI-frame box for a Bevy transform and a local box.
fn obb_of(tf: &GlobalTransform, centre: Vec3, half: Vec3) -> Obb {
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
fn player_boxes(
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

/// Pushes the player's vehicle out of everything it overlaps (fixed step, after the physics).
#[allow(clippy::too_many_arguments)]
pub fn player_collisions(
    index: Res<CollisionIndex>,
    ground: Res<GroundIndex>,
    mut players: Query<PlayerCollide, GroundPlayer>,
    segments: Query<&ArticulatedSegment>,
    traffic: Query<(&GlobalTransform, &VehicleCollider), With<TrafficVehicle>>,
) {
    if std::env::var_os("OPTS_DBG_COLLISION").is_some() {
        tracing::info!(static_boxes = index.len(), "collision index");
    }
    for (mut controller, head, chain, composition) in &mut players {
        // The following parts' boxes: from their parts' `.bus` boxes, at their simulated poses.
        let part_boxes: Vec<BodyBox> = composition
            .map(|c| c.followers.iter().filter_map(|p| p.body).collect())
            .unwrap_or_default();
        let dynamic: Vec<Obb> = traffic
            .iter()
            .map(|(g, c)| obb_of(g, c.centre, c.half))
            .collect();
        let mut push_total = Vec2::ZERO;
        let mut normals = Vec::new();
        for _ in 0..4 {
            let ch = &controller.sim.chassis;
            let followers: Vec<(BodyBox, Vec2, f32)> = chain
                .map(|c| {
                    c.segments
                        .iter()
                        .zip(&part_boxes)
                        .filter_map(|(e, b)| {
                            let (p, h) = segments.get(*e).ok()?.simulated_pose()?;
                            Some((*b, p, h))
                        })
                        .collect()
                })
                .unwrap_or_default();
            let mine = player_boxes(
                ch.position,
                ch.heading_rad,
                ch.elevation_z,
                head,
                &followers,
            );
            let mut worst: Option<(Vec2, f32)> = None;
            for me in &mine {
                for other in index
                    .grid
                    .query(me.centre, me.radius())
                    .chain(dynamic.iter())
                {
                    if !me.overlaps_height(other, STEP_M) {
                        continue;
                    }
                    if let Some(c) = me.contact(other) {
                        if worst.is_none_or(|w| c.depth > w.1) {
                            worst = Some((c.normal, c.depth));
                        }
                    }
                }
            }
            // Steps in the ground around the head's footprint.
            if let Some(n) = ground_step(&ground, &mine[0], ch.elevation_z) {
                if worst.is_none_or(|w| 0.05 > w.1) {
                    worst = Some((n, 0.05));
                }
            }
            let Some((normal, depth)) = worst else { break };
            let push = (depth + 0.01)
                .min(MAX_PUSH_M - push_total.length())
                .max(0.0);
            if push <= 0.0 {
                break;
            }
            controller.sim.chassis.position += normal * push;
            push_total += normal * push;
            normals.push(normal);
        }
        if normals.is_empty() {
            continue;
        }
        // Velocity: nothing into the obstacle, a little scraped off along it.
        let ch = &mut controller.sim.chassis;
        let mut v = ch.world_velocity();
        for n in &normals {
            v = respond(v, *n, RESTITUTION, FRICTION);
        }
        let (s, c) = ch.heading_rad.sin_cos();
        ch.local_velocity = Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c);
        ch.yaw_rate_rad_s *= 0.7;
    }
}

/// Direction away from a step in the ground around a footprint, if there is one: points on the
/// outline are compared with the ground 0.4 m closer to the centre.
fn ground_step(ground: &GroundIndex, body: &Obb, z: f32) -> Option<Vec2> {
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
            if outer - inner > STEP_M && outer > z + STEP_M {
                away += inward;
            }
        }
    }
    (away.length_squared() > 0.0).then(|| away.normalize())
}

/// The index map for `PathBuf` cache keys (kept here so `mod.rs` stays small).
pub(super) type BoundsCache = HashMap<(PathBuf, Option<PathBuf>), Option<(Vec3, Vec3)>>;
