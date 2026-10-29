//! Cone selection pool and depth prepass management for visible light cones.

use super::*;

/// Screen coverage of a cone of `length` and base `radius` whose centre is `distance` away: its
/// projected area up to a constant, `length * radius / distance^2` (distances below 1 m count as
/// 1 m).
#[must_use]
pub fn cone_coverage(length: f32, radius: f32, distance: f32) -> f32 {
    length * radius / (distance * distance).max(1.0)
}

/// The cones that should be visible: the `cap` of `candidates` (cone, coverage) with the
/// largest coverage, where a cone in `current` counts `hysteresis` times its coverage. Ties
/// break by entity, so the choice is stable from one selection to the next.
#[must_use]
pub fn select_cones(
    candidates: &[(Entity, f32)],
    current: &[Entity],
    cap: usize,
    hysteresis: f32,
) -> Vec<Entity> {
    if cap == 0 {
        return Vec::new();
    }
    let mut ranked: Vec<(f32, Entity)> = candidates
        .iter()
        .map(|&(e, s)| {
            (
                if current.contains(&e) {
                    s * hysteresis
                } else {
                    s
                },
                e,
            )
        })
        .collect();
    let better = |a: &(f32, Entity), b: &(f32, Entity)| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1));
    if ranked.len() > cap {
        ranked.select_nth_unstable_by(cap - 1, better);
        ranked.truncate(cap);
    }
    ranked.sort_by(better);
    ranked.into_iter().map(|(_, e)| e).collect()
}

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

/// The brightness a cone reads from its source (`0..=1`).
fn level_of(
    level: ConeLevel,
    glows: &Query<&SceneryLight>,
    spots: &Query<(&SpotLight, &ScriptLight)>,
) -> f32 {
    match level {
        ConeLevel::Fixed(v) => v,
        ConeLevel::Glow(e) => glows.get(e).map_or(0.0, |l| l.level),
        ConeLevel::Spot(e) => spots.get(e).map_or(0.0, |(spot, script)| {
            if script.base_intensity > 0.0 {
                (spot.intensity / script.base_intensity).clamp(0.0, 1.0)
            } else {
                0.0
            }
        }),
    }
}

/// The prepass gate (`0..=1`): 1 without the depth fade; with it, 0 until the camera has had the
/// prepass for [`PREPASS_READY_S`] and [`PREPASS_READY_FRAMES`], then rising over
/// [`CONE_FADE_IN_S`].
#[must_use]
pub fn prepass_gate(depth_fade: bool, age_s: f32, frames: u32) -> f32 {
    if !depth_fade {
        return 1.0;
    }
    if frames < PREPASS_READY_FRAMES || age_s < PREPASS_READY_S {
        return 0.0;
    }
    ((age_s - PREPASS_READY_S) / CONE_FADE_IN_S).clamp(0.0, 1.0)
}

fn test_cone_candidate(
    cone: &LightCone,
    global: &GlobalTransform,
    cam: Vec3,
    fwd: Vec3,
    quality: &LightQuality,
    glows: &Query<&SceneryLight>,
    spots: &Query<(&SpotLight, &ScriptLight)>,
) -> Option<f32> {
    if !cone.kind.allowed(quality) || level_of(cone.level, glows, spots) <= CONE_ON_LEVEL {
        return None;
    }
    let m = global.affine().matrix3;
    let axis = -Vec3::from(m.z_axis);
    let length = axis.length();
    let radius = Vec3::from(m.x_axis).length();
    let apex = global.translation();
    let centre = apex + axis * 0.5;
    let bound = (length * 0.5).max(radius);
    let to_centre = centre - cam;
    if to_centre.length() - bound > cone.kind.max_distance_m() || to_centre.dot(fwd) < -bound {
        return None;
    }
    Some(cone_coverage(length, radius, to_centre.length()))
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

fn update_prepass_gate(
    pool: &mut ConePool,
    quality: &LightQuality,
    camera: Option<(Vec3, Vec3, bool)>,
    dt: f32,
) -> f32 {
    if quality.cone_depth_fade && camera.is_some_and(|c| c.2) {
        pool.prepass_age_s += dt;
        pool.prepass_frames += 1;
    } else {
        pool.prepass_age_s = 0.0;
        pool.prepass_frames = 0;
    }
    let gate = prepass_gate(
        quality.cone_depth_fade,
        pool.prepass_age_s,
        pool.prepass_frames,
    );
    pool.gate = gate;
    gate
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

/// Sets visibility and tag of the active cones for the current light levels; returns how many are
/// lit.
fn apply_levels(p: &mut LightConePoolParams, gate: f32) -> usize {
    let mut lit = 0;
    p.pool.active.retain(|&e| {
        let Ok((cone, mut vis, mut tag)) = p.apply.get_mut(e) else {
            return false;
        };
        let raw = if cone.kind.allowed(&p.quality) {
            level_of(cone.level, &p.glows, &p.spots)
        } else {
            0.0
        };
        if raw > CONE_ON_LEVEL {
            lit += 1;
        }
        let level = raw * gate;
        let on = level > CONE_ON_LEVEL;
        let want = if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        if on {
            let packed = pack_tag(
                cone.shape.color,
                level,
                cone.shape.inner_frac,
                cone.shape.ground_h,
            );
            if tag.value != packed {
                tag.value = packed;
            }
        }
        true
    });
    lit
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

/// Marks a [`DepthPrepass`] that [`cone_depth_prepass_system`] added (so only that one is
/// removed again).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ConeDepthPrepass;

/// Whether the main camera should have the depth prepass: the depth fade is on, cones are
/// allowed at all, and a cone shines (or shone less than [`PREPASS_RELEASE_S`] ago, while the
/// camera already has it).
#[must_use]
pub fn wants_depth_prepass(
    quality: &LightQuality,
    active_cones: usize,
    idle_s: f32,
    has_prepass: bool,
) -> bool {
    let allowed = quality.cone_depth_fade
        && quality.cone_cap > 0
        && (quality.lantern_cones || quality.headlight_cones);
    allowed && (active_cones > 0 || (has_prepass && idle_s < PREPASS_RELEASE_S))
}

/// Adds the depth prepass to the main camera while the cones need it and removes it again.
pub fn cone_depth_prepass_system(
    mut commands: Commands,
    time: Res<Time<Real>>,
    quality: Res<LightQuality>,
    stats: Res<ConeStats>,
    mut pool: ResMut<ConePool>,
    cameras: Query<(Entity, Has<DepthPrepass>, Has<ConeDepthPrepass>), With<crate::MainCamera>>,
) {
    if stats.active > 0 {
        pool.idle_s = 0.0;
    } else {
        pool.idle_s += time.delta_secs();
    }
    for (camera, has, ours) in &cameras {
        let want = wants_depth_prepass(&quality, stats.active, pool.idle_s, has);
        if want && !has {
            commands
                .entity(camera)
                .insert((DepthPrepass, ConeDepthPrepass));
        } else if !want && has && ours {
            commands
                .entity(camera)
                .remove::<(DepthPrepass, ConeDepthPrepass)>();
        }
    }
}
