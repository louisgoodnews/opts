//! The vehicle footprint: its distance and the systems that measure it and find the vehicle above a point.

use super::*;

/// Bounding box of the player vehicle in its own frame (Bevy axes), from its mesh bounds.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct VehicleFootprint {
    /// Smallest corner.
    pub min: Vec3,
    /// Largest corner.
    pub max: Vec3,
}

impl VehicleFootprint {
    /// Distance from the vehicle-frame point `p` to the box (0 inside).
    #[must_use]
    pub fn distance(&self, p: Vec3) -> f32 {
        let d = (self.min - p).max(p - self.max).max(Vec3::ZERO);
        d.length()
    }
}

/// Whether the world point `lamp` hangs over the vehicle: horizontally inside its footprint grown
/// by `margin`, and not below the roof. `vehicle` is the vehicle root's transform.
#[must_use]
pub fn over_vehicle(
    lamp: Vec3,
    vehicle: &GlobalTransform,
    footprint: &VehicleFootprint,
    margin: f32,
) -> bool {
    let p = vehicle.affine().inverse().transform_point3(lamp);
    p.x >= footprint.min.x - margin
        && p.x <= footprint.max.x + margin
        && p.z >= footprint.min.z - margin
        && p.z <= footprint.max.z + margin
        && p.y >= footprint.max.y
}

/// A player vehicle whose footprint is not known yet.
pub(super) type Unmeasured = (With<PlayerVehicle>, Without<VehicleFootprint>);

/// Computes the [`VehicleFootprint`] of the player vehicle from the bounds of its mesh parts
/// (once they exist; retried until then).
pub fn vehicle_footprint_system(
    mut commands: Commands,
    vehicles: Query<(Entity, &GlobalTransform), Unmeasured>,
    children: Query<&Children>,
    parts: Query<(&bevy::camera::primitives::Aabb, &GlobalTransform), Without<PlayerVehicle>>,
) {
    for (vehicle, global) in &vehicles {
        let inverse = global.affine().inverse();
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for e in children.iter_descendants(vehicle) {
            let Ok((aabb, part)) = parts.get(e) else {
                continue;
            };
            let c = Vec3::from(aabb.center);
            let h = Vec3::from(aabb.half_extents);
            for corner in 0..8u8 {
                let s = Vec3::new(
                    if corner & 1 == 0 { -1.0 } else { 1.0 },
                    if corner & 2 == 0 { -1.0 } else { 1.0 },
                    if corner & 4 == 0 { -1.0 } else { 1.0 },
                );
                let local = inverse.transform_point3(part.transform_point(c + h * s));
                lo = lo.min(local);
                hi = hi.max(local);
            }
        }
        if lo.x <= hi.x {
            commands
                .entity(vehicle)
                .insert(VehicleFootprint { min: lo, max: hi });
        }
    }
}
