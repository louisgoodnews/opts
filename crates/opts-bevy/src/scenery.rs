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
    let length = inst.length_m.max(0.01);
    let arc = inst.radius_m.abs() > 1e-3;
    let by_len = (length / 4.0).ceil();
    let by_angle = if arc {
        (length / inst.radius_m.abs()).to_degrees().abs() / 5.0
    } else {
        0.0
    };
    // A straight segment with constant slope is exactly representable by one quad per ribbon.
    let steps = if !arc && (inst.gradient_start - inst.gradient_end).abs() < 1e-6 {
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
mod tests {
    use super::*;
    use opts_core::formats::scenery::{SplineGraphicalProfile, SplineProfilePoint};

    #[test]
    fn test_extrude_spline_profile() {
        let mut profile = SplineProfile::default();
        profile.graphical_profiles.push(SplineGraphicalProfile {
            texture_index: 0,
            points: vec![
                SplineProfilePoint {
                    x: -2.0,
                    z: 0.0,
                    u: 0.0,
                    v_slope: 0.2,
                },
                SplineProfilePoint {
                    x: 2.0,
                    z: 0.0,
                    u: 1.0,
                    v_slope: 0.2,
                },
            ],
        });

        let meshes = extrude_spline_profile(&profile, 10.0);
        assert_eq!(meshes.len(), 1);
        let (tex_idx, mesh) = &meshes[0];
        assert_eq!(*tex_idx, 0);

        // 2 points extruded produces 4 vertices and 2 triangles (6 indices)
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        assert_eq!(positions.len(), 4);
        assert_eq!(mesh.indices().unwrap().len(), 6);
    }

    fn flat_profile() -> SplineProfile {
        let mut p = SplineProfile::default();
        p.graphical_profiles.push(SplineGraphicalProfile {
            texture_index: 0,
            points: vec![
                SplineProfilePoint {
                    x: -2.0,
                    z: 0.0,
                    u: 0.0,
                    v_slope: 0.2,
                },
                SplineProfilePoint {
                    x: 2.0,
                    z: 0.0,
                    u: 1.0,
                    v_slope: 0.2,
                },
            ],
        });
        p
    }

    fn inst(heading: f32, radius: f32) -> MapSplineInstance {
        MapSplineInstance {
            flag: 0,
            sli_file: String::new(),
            id_code: 1,
            conn_id_1: 0,
            conn_id_2: 0,
            position: Vec3::new(10.0, 20.0, 5.0),
            heading_deg: heading,
            length_m: 40.0,
            radius_m: radius,
            gradient_start: 0.0,
            gradient_end: 0.0,
            has_height_profile: false,
            mirrored: false,
            raw_params: Vec::new(),
        }
    }

    #[test]
    fn straight_spline_heads_north_and_faces_up() {
        let meshes = extrude_spline_segment(&flat_profile(), &inst(0.0, 0.0), (300.0, 600.0));
        let mesh = &meshes[0].1;
        let pos: Vec<[f32; 3]> = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap()
            .to_vec();
        // Start: east 310, up 5, north 620 → Bevy (310 ± 2, 5, -620).
        assert!((pos[0][0] - 308.0).abs() < 1e-3 && (pos[1][0] - 312.0).abs() < 1e-3);
        assert!((pos[0][1] - 5.0).abs() < 1e-3);
        assert!((pos[0][2] + 620.0).abs() < 1e-3);
        // End is 40 m further north (more negative Bevy z).
        assert!((pos.last().unwrap()[2] + 660.0).abs() < 1e-3);
        let normals = mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .unwrap()
            .as_float3()
            .unwrap();
        assert!(normals.iter().all(|n| n[1] > 0.99));
    }

    #[test]
    fn heading_90_heads_east() {
        let meshes = extrude_spline_segment(&flat_profile(), &inst(90.0, 0.0), (0.0, 0.0));
        let pos = meshes[0]
            .1
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();
        let last = pos.last().unwrap();
        assert!(
            last[0] > 10.0 + 39.0,
            "heading 90° must run towards +east, got {last:?}"
        );
    }

    #[test]
    fn positive_radius_turns_right() {
        let meshes = extrude_spline_segment(&flat_profile(), &inst(0.0, 50.0), (0.0, 0.0));
        let pos = meshes[0]
            .1
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();
        let end_mid = (pos[pos.len() - 1][0] + pos[pos.len() - 2][0]) / 2.0;
        assert!(
            end_mid > 10.0,
            "positive radius curves towards east (right) when heading north"
        );
    }

    #[test]
    fn plain_spline_rises_by_the_average_of_its_percent_gradients() {
        let mut inst = inst(0.0, 0.0);
        inst.length_m = 30.2;
        inst.gradient_start = 0.07;
        inst.gradient_end = 0.06;
        let z = height_offset(&inst, 30.2, 30.2);
        assert!((z - 1.963).abs() < 1e-3, "{z}");
    }

    #[test]
    fn spline_h_reaches_its_explicit_height_with_a_smooth_ramp() {
        let mut inst = inst(0.0, 0.0);
        inst.length_m = 10.0;
        inst.has_height_profile = true;
        inst.raw_params = vec!["0".into(), "0".into(), "0.5".into()];
        assert!((height_offset(&inst, 10.0, 10.0) - 0.5).abs() < 1e-6);
        assert!(
            (height_offset(&inst, 5.0, 10.0) - 0.25).abs() < 1e-6,
            "S-curve midpoint"
        );
        assert!(height_offset(&inst, 1.0, 10.0) < 0.05, "starts flat");
    }
}

#[cfg(test)]
mod pose_tests {
    use super::*;

    fn straight() -> MapSplineInstance {
        MapSplineInstance {
            flag: 0,
            sli_file: String::new(),
            id_code: 1,
            conn_id_1: 0,
            conn_id_2: 0,
            position: Vec3::new(10.0, 20.0, 5.0),
            heading_deg: 90.0,
            length_m: 100.0,
            radius_m: 0.0,
            gradient_start: 0.02,
            gradient_end: 0.02,
            has_height_profile: false,
            mirrored: false,
            raw_params: vec![],
        }
    }

    #[test]
    fn straight_pose_heads_east_and_climbs() {
        let p = spline_pose(&straight(), 50.0);
        assert!((p.x - 60.0).abs() < 1e-3 && (p.y - 20.0).abs() < 1e-3);
        assert!((p.z - 6.0).abs() < 1e-4);
    }

    #[test]
    fn right_turn_arc_ends_south_of_east_start() {
        let mut inst = straight();
        inst.radius_m = 100.0;
        inst.length_m = 200.0;
        let p = spline_pose(&inst, 100.0 * std::f32::consts::FRAC_PI_2);
        // Heading east, turning right by 90° ends heading south, 100 m east and 100 m south.
        assert!((p.x - 110.0).abs() < 0.05 && (p.y - -80.0).abs() < 0.05);
    }
}

#[cfg(test)]
mod mirror_tests {
    use super::*;
    use opts_core::formats::scenery::sli::{SplineGraphicalProfile, SplineProfilePoint};

    fn x_extent(mirrored: bool) -> (f32, f32) {
        let pt = |x: f32| SplineProfilePoint {
            x,
            z: 0.0,
            u: 0.0,
            v_slope: 1.0,
        };
        let profile = SplineProfile {
            graphical_profiles: vec![SplineGraphicalProfile {
                texture_index: 0,
                points: vec![pt(0.0), pt(4.0)],
            }],
            ..Default::default()
        };
        let mut inst = pose_tests_instance();
        inst.heading_deg = 0.0; // north: right is +x
        inst.position = Vec3::ZERO;
        inst.mirrored = mirrored;
        let meshes = extrude_spline_segment(&profile, &inst, (0.0, 0.0));
        let positions = meshes[0]
            .1
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3().map(<[_]>::to_vec))
            .unwrap();
        let xs: Vec<f32> = positions.iter().map(|p| p[0]).collect();
        (
            xs.iter().copied().fold(f32::MAX, f32::min),
            xs.iter().copied().fold(f32::MIN, f32::max),
        )
    }

    fn pose_tests_instance() -> MapSplineInstance {
        MapSplineInstance {
            flag: 0,
            sli_file: String::new(),
            id_code: 1,
            conn_id_1: 0,
            conn_id_2: 0,
            position: Vec3::ZERO,
            heading_deg: 0.0,
            length_m: 10.0,
            radius_m: 0.0,
            gradient_start: 0.0,
            gradient_end: 0.0,
            has_height_profile: false,
            mirrored: false,
            raw_params: vec![],
        }
    }

    #[test]
    fn mirrored_spline_flips_the_profile_to_the_left() {
        assert_eq!(x_extent(false), (0.0, 4.0));
        assert_eq!(x_extent(true), (-4.0, 0.0));
    }
}
