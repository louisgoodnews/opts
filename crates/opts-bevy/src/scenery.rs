//! Bevy asset loaders and entity spawning bridge for OMSI scenery objects (`.sco`)
//! and spline profiles (`.sli`).
//!
//! Bridges parsed scenery assets into Bevy's ECS hierarchy and procedural
//! spline extrusion meshes.

use bevy::asset::io::Reader;
use bevy::asset::{Asset, AssetLoader, Handle, LoadContext};
use bevy::ecs::entity::Entity;
use bevy::ecs::name::Name;
use bevy::ecs::system::Commands;
use bevy::mesh::Mesh3d;
use bevy::pbr::MeshMaterial3d;
use bevy::pbr::StandardMaterial;

use bevy::mesh::Mesh;
use bevy::reflect::TypePath;
use bevy::transform::components::Transform;
use thiserror::Error;

use crate::coords::OmsiTransform;
use bevy::math::Vec3;
use opts_core::formats::map::MapSplineInstance;
use opts_core::formats::scenery::{SceneryObject, SceneryParseError, SplineProfile};

use crate::mesh::smooth_normal_mesh;

/// Errors that can occur when loading scenery object or spline assets.
#[derive(Debug, Error)]
pub enum SceneryLoaderError {
    /// An I/O error occurred while reading from the asset source.
    #[error("I/O error reading scenery asset: {0}")]
    Io(#[from] std::io::Error),

    /// Parsing failed within `opts-core`.
    #[error("Failed to parse scenery asset: {0}")]
    Parse(#[from] SceneryParseError),
}

/// Bevy [`Asset`] wrapper for a parsed OMSI scenery object definition (`.sco`).
#[derive(Asset, TypePath, Debug, Clone)]
pub struct SceneryObjectAsset {
    /// Parsed scenery object manifest.
    pub object: SceneryObject,
}

/// Bevy [`AssetLoader`] for OMSI `.sco` scenery object files.
#[derive(Default, TypePath)]
pub struct ScoAssetLoader;

impl AssetLoader for ScoAssetLoader {
    type Asset = SceneryObjectAsset;
    type Settings = ();
    type Error = SceneryLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let object = SceneryObject::parse(&bytes)?;
        Ok(SceneryObjectAsset { object })
    }

    fn extensions(&self) -> &[&str] {
        &["sco"]
    }
}

/// Bevy [`Asset`] wrapper for a parsed OMSI spline cross-section profile (`.sli`).
#[derive(Asset, TypePath, Debug, Clone)]
pub struct SplineProfileAsset {
    /// Parsed spline profile.
    pub profile: SplineProfile,
}

/// Bevy [`AssetLoader`] for OMSI `.sli` spline profile files.
#[derive(Default, TypePath)]
pub struct SliAssetLoader;

impl AssetLoader for SliAssetLoader {
    type Asset = SplineProfileAsset;
    type Settings = ();
    type Error = SceneryLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let profile = SplineProfile::parse(&bytes)?;
        Ok(SplineProfileAsset { profile })
    }

    fn extensions(&self) -> &[&str] {
        &["sli"]
    }
}

/// Extrudes a 2D spline profile along a straight segment of length `length_m`
/// producing Bevy [`Mesh`] primitives partitioned by texture slot.
///
/// Convenience wrapper over [`extrude_spline_segment`] for a segment starting at the origin and
/// heading north (Bevy `-Z`). Returns `(texture_index, Mesh)` per graphical profile ribbon.
#[must_use]
pub fn extrude_spline_profile(profile: &SplineProfile, length_m: f32) -> Vec<(usize, Mesh)> {
    let segment = MapSplineInstance {
        flag: 0,
        sli_file: String::new(),
        id_code: 0,
        conn_id_1: 0,
        conn_id_2: 0,
        position: Vec3::ZERO,
        heading_deg: 0.0,
        length_m,
        radius_m: 0.0,
        gradient_start: 0.0,
        gradient_end: 0.0,
        has_height_profile: false,
        mirrored: false,
        raw_params: Vec::new(),
    };
    extrude_spline_segment(profile, &segment, (0.0, 0.0))
}

/// Height above the segment start at arc length `s`.
///
/// `[spline_h]` records state the total height change explicitly (parameter 13); the profile is
/// then a cubic Hermite curve between the end gradients (a smooth ramp). Plain `[spline]` records
/// change the gradient linearly from start to end.
fn height_offset(inst: &MapSplineInstance, s: f32, length: f32) -> f32 {
    debug_assert!((length - inst.length_m.max(0.01)).abs() < 1e-3);
    inst.height_offset(s)
}

/// Centreline pose of a placed spline at arc length `s`, in the spline's tile-local OMSI frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplinePose {
    /// Easting (m, tile-local).
    pub x: f32,
    /// Northing (m, tile-local).
    pub y: f32,
    /// Elevation (m, absolute).
    pub z: f32,
    /// Heading in radians, clockwise from north.
    pub heading: f32,
}

/// Pose on the centreline of `inst` at arc length `s` (clamped to the segment).
#[must_use]
pub fn spline_pose(inst: &MapSplineInstance, s: f32) -> SplinePose {
    let length = inst.length_m.max(0.01);
    let s = s.clamp(0.0, length);
    let phi = inst.heading_deg.to_radians();
    let (heading, x, y) = if inst.radius_m.abs() > 1e-3 {
        let r = inst.radius_m;
        let h = phi + s / r;
        (
            h,
            inst.position.x - r * (h.cos() - phi.cos()),
            inst.position.y + r * (h.sin() - phi.sin()),
        )
    } else {
        (
            phi,
            inst.position.x + s * phi.sin(),
            inst.position.y + s * phi.cos(),
        )
    };
    SplinePose {
        x,
        y,
        z: inst.position.z + height_offset(inst, s, length),
        heading,
    }
}

/// The cross-sections a placed spline is drawn at: `(east, north, elevation, heading, s)` in the
/// frame whose origin is `tile_origin` (the spline's tile corner in that frame). Straight
/// segments with a constant slope need only their two ends; arcs and changing gradients
/// (including `[spline_h]` ramps) are sampled every 4 m or 5°. The terrain carve builds its
/// ribbons on the same stations so the carved ground matches the drawn surface.
#[must_use]
pub fn spline_stations(
    inst: &MapSplineInstance,
    tile_origin: (f32, f32),
) -> Vec<(f32, f32, f32, f32, f32)> {
    let length = inst.length_m.max(0.01);
    let arc = inst.radius_m.abs() > 1e-3;
    let by_len = (length / 4.0).ceil();
    let by_angle = if arc {
        (length / inst.radius_m.abs()).to_degrees().abs() / 5.0
    } else {
        0.0
    };
    // A straight segment with constant slope is exactly representable by one quad per ribbon;
    // a `[spline_h]` ramp is a cubic even between equal end gradients.
    let ramp = [0.25, 0.5, 0.75].iter().any(|f| {
        (height_offset(inst, length * f, length) - inst.gradient_start * length * f).abs() > 1e-4
    });
    let steps = if !arc && !ramp && (inst.gradient_start - inst.gradient_end).abs() < 1e-6 {
        1
    } else {
        by_len.max(by_angle.ceil()).clamp(1.0, 400.0) as usize
    };

    let phi = inst.heading_deg.to_radians();
    let p0 = (
        tile_origin.0 + inst.position.x,
        tile_origin.1 + inst.position.y,
    );

    // Centreline samples: (east, north, elevation, heading).
    let mut samples = Vec::with_capacity(steps + 1);
    for k in 0..=steps {
        let s = length * k as f32 / steps as f32;
        let (h, ex, ny) = if arc {
            let r = inst.radius_m;
            let h = phi + s / r;
            (
                h,
                p0.0 - r * (h.cos() - phi.cos()),
                p0.1 + r * (h.sin() - phi.sin()),
            )
        } else {
            (phi, p0.0 + s * phi.sin(), p0.1 + s * phi.cos())
        };
        let z = inst.position.z + height_offset(inst, s, length);
        samples.push((ex, ny, z, h, s));
    }

    samples
}

/// Extrudes a `.sli` graphical profile along a placed spline segment.
///
/// The segment starts at `inst.position` (tile-local), heads along `inst.heading_deg`
/// (0° = north, **clockwise** positive like a compass), follows the arc given by
/// `inst.radius_m` (positive radius = heading increases = turns right) and blends the
/// longitudinal slope from `gradient_start` to `gradient_end`. Both sign conventions were
/// derived from the corpus: the end of a placed segment coincides with the start of the
/// segment it connects to only under these conventions.
///
/// Returns `(texture_index, mesh)` per graphical profile, in absolute Bevy world coordinates.
#[must_use]
pub fn extrude_spline_segment(
    profile: &SplineProfile,
    inst: &MapSplineInstance,
    tile_origin: (f32, f32),
) -> Vec<(usize, Mesh)> {
    let samples = spline_stations(inst, tile_origin);
    let mut out = Vec::new();
    for gp in &profile.graphical_profiles {
        if gp.points.len() < 2 {
            continue;
        }
        let n = gp.points.len();
        let mut positions = Vec::with_capacity(n * samples.len());
        let mut uvs = Vec::with_capacity(n * samples.len());
        // A mirrored spline lays its profile out with x negated.
        let side = if inst.mirrored { -1.0 } else { 1.0 };
        for &(ex, ny, z, h, s) in &samples {
            let (rx, ry) = (h.cos(), -h.sin());
            for p in &gp.points {
                let x = p.x * side;
                positions.push([ex + rx * x, z + p.z, -(ny + ry * x)]);
                uvs.push([p.u, s * p.v_slope]);
            }
        }
        let mut indices = Vec::new();
        for k in 0..samples.len() - 1 {
            for i in 0..n - 1 {
                let a = (k * n + i) as u32;
                let b = a + 1;
                let c = a + n as u32;
                let d = c + 1;
                if inst.mirrored {
                    indices.extend_from_slice(&[a, c, b, b, c, d]);
                } else {
                    indices.extend_from_slice(&[a, b, c, b, d, c]);
                }
            }
        }
        out.push((
            gp.texture_index,
            smooth_normal_mesh(positions, uvs, indices),
        ));
    }
    out
}

/// Spawns an OMSI scenery object entity hierarchy in Bevy.
///
/// Creates a parent entity carrying an [`OmsiTransform`] and `Transform`/`Visibility`,
/// along with child entities for visual 3D mesh attachments.
pub fn spawn_scenery_object(
    commands: &mut Commands,
    sco: &SceneryObject,
    transform: Transform,
    mesh_handles: &[Handle<Mesh>],
    material_handles: &[Handle<StandardMaterial>],
) -> Entity {
    let name = sco
        .friendly_name
        .clone()
        .unwrap_or_else(|| "SceneryObject".to_string());

    let parent = commands
        .spawn((
            Name::new(name),
            transform,
            OmsiTransform::from_bevy_transform(&transform),
        ))
        .id();

    for (i, mesh_def) in sco.meshes.iter().enumerate() {
        if let Some(mesh_handle) = mesh_handles.get(i) {
            let mat_handle = material_handles.get(i).cloned().unwrap_or_default();

            let child = commands
                .spawn((
                    Mesh3d(mesh_handle.clone()),
                    MeshMaterial3d(mat_handle),
                    Transform::IDENTITY,
                ))
                .insert(Name::new(format!("Mesh_{i}_{}", mesh_def.file)))
                .id();

            commands.entity(parent).add_children(&[child]);
        }
    }

    parent
}

#[cfg(test)]
mod tests;
