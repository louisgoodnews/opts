//! Traffic vehicle variants, lane budget and slot choice.

use super::*;

/// Reads the map's AI lists once and parses one vehicle type every few frames.
pub fn load_traffic_variants(
    job: Option<ResMut<WorldLoadJob>>,
    settings: Res<TrafficSettings>,
    mut pool: ResMut<TrafficPool>,
) {
    let Some(job) = job else { return };
    if settings.max == 0 {
        return;
    }
    if !pool.started {
        pool.started = true;
        pool.rng = Some(Rng::new(0xA11_CA45));
        let PoolSetup {
            groups,
            slots,
            defaults,
        } = read_groups(&job);
        pool.groups = groups;
        pool.slots = slots;
        pool.defaults = defaults;
        pool.road_curve = DensityProfile {
            days: 0,
            points: job.manifest.traffic_density_road.clone(),
        };
    }
    pool.frames += 1;
    if !pool.frames.is_multiple_of(10) {
        return;
    }
    let Some(i) = pool
        .slots
        .iter()
        .position(|s| matches!(s.state, SlotState::Pending))
    else {
        return;
    };
    let file = pool.slots[i].file.clone();
    match load_variant(&job, &file) {
        Ok(v) => pool.slots[i].state = SlotState::Ready(Box::new(v)),
        Err(e) => {
            tracing::debug!(vehicle = %file, "AI vehicle skipped: {e}");
            pool.slots[i].state = SlotState::Failed;
        }
    }
}

/// The meshes of one unit: the plain `[LOD] 0` body for cars; for buses the full-detail group
/// seen from outside, with the AI-only meshes (`[viewpoint]` 4: interiors of the AI model) and
/// without the cockpit-only ones.
pub(super) fn unit_lod(unit: &Unit) -> Option<ModelLod> {
    if unit.profile.has_cabin {
        unit.model.detail_lod().map(|mut lod| {
            lod.meshes.retain(|m| {
                let raw = m.viewpoint.raw();
                raw == 0 || raw & 1 != 0 || raw & 4 != 0
            });
            for m in &mut lod.meshes {
                if m.viewpoint.raw() & 4 != 0 {
                    m.viewpoint = opts_core::formats::vehicles::model::ViewpointFlags::ALWAYS;
                }
            }
            lod
        })
    } else {
        unit.model
            .lods
            .iter()
            .find(|l| l.threshold == 0.0)
            .or_else(|| unit.model.lods.first())
            .cloned()
    }
}

/// Time factor of a group: its `unsched_trafficdens.txt` factor times its curve for the hour and
/// day (`[set_day_of_week]`), or the map's `[trafficdensity_road]` curve when the group has no
/// entry there. Unclamped: values above 1 mean denser traffic.
pub(in super::super::super) fn time_factor(
    group: &PoolGroup,
    road_curve: &DensityProfile,
    day_bit: u8,
    hour: f32,
) -> f64 {
    f64::from(match &group.density {
        Some(d) => d.density(day_bit, hour),
        None => road_curve.at(hour),
    })
    .max(0.0)
}

/// Lane metres inside the disc of `radius` around `centre`, weighted by each pool group's
/// density on the lane: `Σ_lanes length_inside × D(lane, group)`.
pub(in super::super::super) fn group_lane_metres(
    graph: &LaneGraph,
    centre: DVec2,
    radius: f64,
    groups: &[PoolGroup],
    defaults: &[u32],
) -> Vec<f64> {
    const SAMPLES: usize = 8;
    let mut out = vec![0.0; groups.len()];
    for lane in graph.lane_ids().filter_map(|l| graph.lane(l)) {
        if lane.kind != LaneKind::Road {
            continue;
        }
        let len = lane.length();
        let inside = (0..=SAMPLES)
            .filter(|&i| {
                lane.geometry
                    .point_at(len * i as f64 / SAMPLES as f64)
                    .distance(centre)
                    <= radius
            })
            .count();
        if inside == 0 {
            continue;
        }
        let metres = len * inside as f64 / (SAMPLES + 1) as f64;
        for (g, group) in groups.iter().enumerate() {
            out[g] += metres * lane.rules.density.for_group(group.density_group, defaults);
        }
    }
    out
}

/// Picks a group that is below its target (weighted by how far below) and has a vehicle ready,
/// then a vehicle of it by weight.
pub(super) fn pick_slot(pool: &TrafficPool, deficits: &[f64], rng: &mut Rng) -> Option<usize> {
    let ready = |s: &Slot| matches!(s.state, SlotState::Ready(_)) && s.weight > 0.0;
    let live: Vec<usize> = (0..pool.groups.len())
        .filter(|&g| deficits[g] > 0.0 && pool.slots.iter().any(|s| s.group == g && ready(s)))
        .collect();
    let total: f64 = live.iter().map(|&g| deficits[g]).sum();
    let mut pick = rng.next_f64() * total;
    let mut group = *live.first()?;
    for &g in &live {
        if pick < deficits[g] {
            group = g;
            break;
        }
        pick -= deficits[g];
    }
    let in_group: Vec<usize> = (0..pool.slots.len())
        .filter(|&i| pool.slots[i].group == group && ready(&pool.slots[i]))
        .collect();
    let weight: f32 = in_group.iter().map(|&i| pool.slots[i].weight).sum();
    let mut pick = rng.next_f64() as f32 * weight;
    let mut slot = *in_group.first()?;
    for &i in &in_group {
        if pick < pool.slots[i].weight {
            slot = i;
            break;
        }
        pick -= pool.slots[i].weight;
    }
    Some(slot)
}
