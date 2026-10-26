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

mod mesh_boxes;
mod player;

pub use player::player_collisions;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use opts_bevy::{ArticulatedChain, ArticulatedSegment, PlayerVehicle, VehicleController};
use opts_core::formats::scenery::BaseSurface;
use opts_sim::collision::{ColliderGrid, Obb};

use super::ground_height::GroundIndex;
use super::rail::VehicleComposition;
use super::traffic::TrafficVehicle;
use super::{RailPlayer, WorldLoadJob};

/// Largest half extents `(right, up, back)` the box of an AI vehicle may have: 3.6 m wide, 5 m
/// high, 20 m long.
const MAX_VEHICLE_HALF: Vec3 = Vec3::new(1.8, 2.5, 10.0);
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

    /// Registers a static box under the key of its tile (tests and tools; tiles register their
    /// objects themselves).
    pub fn insert_box(&mut self, tile: (i32, i32), body: Obb) {
        self.grid.insert(tile, body);
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
    /// The boxes of a collision mesh in Bevy's local axes, cached (one box for a small mesh, a
    /// set that follows the shape for a large one: see [`mesh_boxes`]); `None` for unreadable
    /// (protected) or missing meshes.
    ///
    /// `bend` is the crossing base the object is bent over (`[crossing_heightdeformation]`).
    pub(super) fn collision_bounds(
        &mut self,
        model_dir: &Path,
        rel: &str,
        bend: Option<&(PathBuf, Arc<BaseSurface>)>,
    ) -> Option<Vec<(Vec3, Vec3)>> {
        let path = self.install.find_in(model_dir, rel)?;
        let key = (path.clone(), bend.map(|b| b.0.clone()));
        if let Some(b) = self.collision_bounds.get(&key) {
            return b.clone();
        }
        let bounds = (|| {
            let mut models = opts_bevy::load_decoded_meshes(&path).ok()?;
            if let Some((_, base)) = bend {
                for m in &mut models {
                    base.bend_mesh(m);
                }
            }
            let (mut positions, mut indices): (Vec<Vec3>, Vec<u32>) = (Vec::new(), Vec::new());
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
                    let base = positions.len() as u32;
                    positions.extend(pos.iter().map(|p| Vec3::from(*p)));
                    match mesh.indices() {
                        Some(bevy::mesh::Indices::U32(ix)) => {
                            indices.extend(ix.iter().map(|i| base + i));
                        }
                        Some(bevy::mesh::Indices::U16(ix)) => {
                            indices.extend(ix.iter().map(|i| base + u32::from(*i)));
                        }
                        None => indices.extend(base..base + pos.len() as u32),
                    }
                }
            }
            let boxes = mesh_boxes::boxes_of(&positions, &indices);
            (!boxes.is_empty()).then_some(boxes)
        })();
        self.collision_bounds.insert(key, bounds.clone());
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
        let Some(parts) = job.collision_bounds(model_dir, rel, bend) else {
            continue;
        };
        for (lo, hi) in parts {
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
    }
    if boxes.is_empty() {
        return;
    }
    // An object marked `[surface]` is a drivable ground mesh: its collision mesh is what the
    // vehicles drive on (a junction or street plate a few decimetres over the terrain), not a
    // wall; it went into the obstacle index and stopped buses at the first plate (Köln,
    // Frankenstraße).
    if sco.surface {
        // The plate's textures under the wheels (M-0146).
        let tris = super::plate_surf::plate_triangles(job, sco, model_dir, tf, bend);
        commands.queue(move |world: &mut World| {
            if let Some(mut ground) = world.get_resource_mut::<GroundIndex>() {
                ground.insert_plates(key, boxes);
                ground.insert_plate_triangles(key, tris);
            }
        });
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
    bounds: Query<(
        &Aabb,
        &GlobalTransform,
        Has<opts_bevy::light_cones::LightCone>,
    )>,
) {
    for (e, root) in &vehicles {
        let inverse = root.affine().inverse();
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        let mut count = 0;
        for d in children.iter_descendants(e) {
            let Ok((aabb, g, is_cone)) = bounds.get(d) else {
                continue;
            };
            // The headlight cones reach 8 m out of a car; they are light, not body.
            if is_cone {
                continue;
            }
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
            // A road vehicle is at most so wide, high and long; more is helper geometry that got
            // through (a car measured 29 by 34 m once and pushed the player about).
            let half = ((hi - lo) * 0.5).min(MAX_VEHICLE_HALF);
            commands.entity(e).insert(VehicleCollider {
                centre: (lo + hi) * 0.5,
                half,
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

/// The index map for `PathBuf` cache keys (kept here so `mod.rs` stays small).
pub(super) type BoundsCache = HashMap<(PathBuf, Option<PathBuf>), Option<Vec<(Vec3, Vec3)>>>;

/// Hands the AI traffic the footprints of the player's road vehicles (head and following parts),
/// so the agents stop behind them or change lanes round them (`opts_traffic::obstacles`) instead
/// of driving into them; the parked vehicles near the camera count too.
pub fn publish_traffic_obstacles(
    mut state: ResMut<super::traffic::TrafficState>,
    players: Query<PlayerCollide, GroundPlayer>,
    segments: Query<&ArticulatedSegment>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
) {
    let mut obstacles = vec![];
    for (controller, head, chain, composition) in &players {
        let part_boxes: Vec<BodyBox> = composition
            .map(|c| c.followers.iter().filter_map(|p| p.body).collect())
            .unwrap_or_default();
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
        let ch = &controller.sim.chassis;
        let v = ch.world_velocity();
        let boxes = player_boxes(
            ch.position,
            ch.heading_rad,
            ch.elevation_z,
            head,
            &followers,
        );
        obstacles.extend(boxes.into_iter().map(|b| opts_traffic::Obstacle {
            centre: bevy::math::DVec2::new(f64::from(b.centre.x), f64::from(b.centre.y)),
            heading: f64::from(b.heading),
            length: f64::from(b.half.y) * 2.0,
            width: f64::from(b.half.x) * 2.0,
            velocity: bevy::math::DVec2::new(f64::from(v.x), f64::from(v.y)),
        }));
    }
    state.set_obstacles_with_parked(obstacles, camera.single().ok());
}

#[cfg(test)]
mod collider_tests {
    use super::*;
    use bevy::camera::primitives::Aabb;

    /// A car whose meshes include helper geometry far out of the body (8 m headlight cones used
    /// to) still gets a box of vehicle size: the AI car that measured 29 by 34 m pushed the
    /// player's standing bus 40 m across the map.
    #[test]
    fn a_vehicle_box_is_never_larger_than_a_vehicle() {
        let mut app = App::new();
        app.add_systems(Update, attach_traffic_colliders);
        let root = app
            .world_mut()
            .spawn((
                super::super::traffic::TrafficVehicle::at(bevy::math::DVec2::ZERO, 0.0, 0.0),
                GlobalTransform::default(),
            ))
            .id();
        for (centre, half) in [
            (Vec3::ZERO, Vec3::new(0.9, 0.7, 2.2)),
            (Vec3::new(0.0, 0.5, 0.5), Vec3::new(0.8, 0.5, 1.5)),
            (Vec3::new(0.0, 0.3, -0.5), Vec3::new(0.7, 0.3, 1.0)),
            // Something that reaches 8 m out sideways and 12 m forwards.
            (Vec3::new(8.0, 0.0, 12.0), Vec3::new(8.5, 8.5, 8.5)),
        ] {
            let child = app
                .world_mut()
                .spawn((
                    Aabb {
                        center: centre.into(),
                        half_extents: half.into(),
                    },
                    GlobalTransform::default(),
                ))
                .id();
            app.world_mut().entity_mut(root).add_child(child);
        }
        app.update();
        let c = app
            .world()
            .get::<VehicleCollider>(root)
            .expect("the box was measured");
        assert!(c.half.x <= MAX_VEHICLE_HALF.x, "{:?}", c.half);
        assert!(c.half.y <= MAX_VEHICLE_HALF.y, "{:?}", c.half);
        assert!(c.half.z <= MAX_VEHICLE_HALF.z, "{:?}", c.half);
    }
}
