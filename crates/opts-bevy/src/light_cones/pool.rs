//! Cone selection pool and depth prepass management for visible light cones.

use super::*;

mod levels;
mod prepass;
mod selection;

use levels::*;
pub use prepass::*;
pub use selection::*;

/// The cones the pool made visible, and when it last chose.
#[derive(Resource, Debug, Default)]
pub struct ConePool {
    /// Visible cones (lit and chosen), nearest-by-coverage first at the last selection.
    pub active: Vec<Entity>,
    /// Seconds since the last selection.
    since_select_s: f32,
    /// Quality values at the last selection (lantern switch, headlight switch, cap).
    key: Option<(bool, bool, usize)>,
    /// Seconds since a cone last shone (for the depth prepass).
    idle_s: f32,
    /// Seconds and frames the main camera has had the depth prepass while the fade is on.
    prepass_age_s: f32,
    prepass_frames: u32,
    /// The gate: cones fade in only once the prepass is ready (`0..=1`; 1 with the fade off).
    pub gate: f32,
}

/// What the last selection saw (for `sim.get` and tests).
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct ConeStats {
    /// Cone entities in the world.
    pub total: usize,
    /// Of those, lit, allowed by the settings and within range in front of the camera.
    pub eligible: usize,
    /// Visible now (within the budget and lit).
    pub active: usize,
    /// Eligible but hidden because of the cap.
    pub over_budget: usize,
    /// Microseconds the last selection took.
    pub select_us: u32,
}

/// System parameters for the light cone pooling system.
#[derive(bevy::ecs::system::SystemParam)]
pub struct LightConePoolParams<'w, 's> {
    pub time: Res<'w, Time<Real>>,
    pub quality: Res<'w, LightQuality>,
    pub pool: ResMut<'w, ConePool>,
    pub stats: ResMut<'w, ConeStats>,
    pub cameras: Query<
        'w,
        's,
        (&'static Camera, &'static GlobalTransform, Has<DepthPrepass>),
        With<crate::MainCamera>,
    >,
    pub scan: Query<'w, 's, (Entity, &'static LightCone, &'static GlobalTransform)>,
    pub apply: Query<
        'w,
        's,
        (
            &'static LightCone,
            &'static mut Visibility,
            &'static mut MeshTag,
        ),
    >,
    pub glows: Query<'w, 's, &'static SceneryLight>,
    pub spots: Query<'w, 's, (&'static SpotLight, &'static ScriptLight)>,
}

/// Picks the cones that shine now (the nearest within the budget, with hysteresis), hides the
/// ones that dropped out and records the statistics.
fn reselect(p: &mut LightConePoolParams, camera: Option<(Vec3, Vec3)>, cap: usize) {
    let started = std::time::Instant::now();
    let mut candidates: Vec<(Entity, f32)> = Vec::new();
    let mut total = 0;
    if let Some((cam, fwd)) = camera.filter(|_| cap > 0) {
        for (e, cone, global) in &p.scan {
            total += 1;
            if let Some(cov) =
                test_cone_candidate(cone, global, cam, fwd, &p.quality, &p.glows, &p.spots)
            {
                candidates.push((e, cov));
            }
        }
    } else {
        total = p.scan.iter().count();
    }
    let chosen = select_cones(&candidates, &p.pool.active, cap, CONE_HYSTERESIS);
    for e in p.pool.active.iter().filter(|e| !chosen.contains(e)) {
        if let Ok((_, mut vis, _)) = p.apply.get_mut(*e) {
            if *vis != Visibility::Hidden {
                *vis = Visibility::Hidden;
            }
        }
    }
    *p.stats = ConeStats {
        total,
        eligible: candidates.len(),
        active: chosen.len(),
        over_budget: candidates.len().saturating_sub(chosen.len()),
        select_us: started.elapsed().as_micros().min(u128::from(u32::MAX)) as u32,
    };
    p.pool.active = chosen;
}

/// Chooses the visible cones every [`SELECT_INTERVAL_S`] (at once when the quality settings
/// change), and keeps their brightness and visibility current every frame.
pub fn light_cone_pool_system(mut p: LightConePoolParams) {
    let cap = p.quality.cone_cap.min(MAX_CONE_CAP) as usize;
    let key = (p.quality.lantern_cones, p.quality.headlight_cones, cap);
    p.pool.since_select_s += p.time.delta_secs();
    let camera = p
        .cameras
        .iter()
        .find(|(c, _, _)| c.is_active)
        .map(|(_, t, pre)| (t.translation(), t.forward().as_vec3(), pre));
    let gate = update_prepass_gate(&mut p.pool, &p.quality, camera, p.time.delta_secs());
    if p.pool.since_select_s >= SELECT_INTERVAL_S || p.pool.key != Some(key) {
        p.pool.since_select_s = 0.0;
        p.pool.key = Some(key);
        reselect(&mut p, camera.map(|c| (c.0, c.1)), cap);
    }
    let lit = apply_levels(&mut p, gate);
    if p.stats.active != lit {
        p.stats.active = lit;
    }
}
