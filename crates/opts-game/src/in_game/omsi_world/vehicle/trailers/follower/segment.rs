//! The articulated segment of a following unit and the load of the unit in the chain.

use super::super::super::super::consist_plan::PlannedPart;
use super::*;

/// The articulated segment of a unit: its joint to the part in front, limits, load and wheels.
pub(super) fn articulated_segment(
    part: &PlannedPart,
    manifest: &BusManifest,
    load: &opts_sim::UnitLoad,
    axle_base: usize,
) -> opts_bevy::ArticulatedSegment {
    let reversed = part.turned;
    // The link to the part in front inside its unit (a unit's first file has none: only rail
    // places those, by `behind`, and the articulated segment is unused there).
    let (b, f) = part
        .joint
        .unwrap_or((Vec3::new(0.0, -part.behind, 0.0), Vec3::ZERO));
    let rear_axle_y = rear_axle_y(manifest, reversed, f.y);
    let max_alpha = manifest
        .coupling_front_limits
        .map_or(52.5, |l| l.max_alpha_deg);
    let mut s = opts_bevy::ArticulatedSegment::new(
        Vec2::new(b.x, b.y),
        Vec2::new(f.x, f.y),
        rear_axle_y,
        max_alpha,
    );
    s.reversed = reversed;
    s.front_z = f.z;
    s.load = Some(load.clone());
    // The bus files list min and max in either order (3 files have 20,-20).
    s.beta_limits_deg = manifest.coupling_front_limits.map(|l| {
        (
            l.min_beta_deg.min(l.max_beta_deg),
            l.min_beta_deg.max(l.max_beta_deg),
        )
    });
    s.set_wheels(
        axle_base,
        manifest
            .axles
            .iter()
            .map(|a| a.wheel_diameter_m.filter(|&d| d > 0.1).unwrap_or(0.98) * 0.5),
    );
    s
}

/// The unit's load, and the running masses of the chain updated with it.
pub(super) fn account_load(
    st: &mut ChainState,
    part: &PlannedPart,
    rail: bool,
) -> opts_sim::UnitLoad {
    let manifest = &part.manifest;
    // Road trailers assume the mass of the unit in front when they state none; rail cars
    // (bogies, joints and couplers are vehicles of their own and state 0) add what they state.
    let fallback_kg = if rail { 0.0 } else { st.previous_kg };
    if !rail && manifest_mass_kg(manifest).is_none() {
        tracing::warn!(trailer = %part.path.display(), assumed_kg = st.previous_kg, "trailer states no mass; the mass of the unit in front is used");
    }
    let load = opts_sim::UnitLoad::from_manifest_or(manifest, part.turned, fallback_kg);
    st.previous_kg = load.mass_kg;
    if rail {
        st.total_kg += load.mass_kg;
        if part.powered {
            st.powered_kg += load.mass_kg;
        }
    }
    load
}
