//! Traffic vehicle variants, lane budget and slot choice.

use super::*;

/// Reads the map's AI lists once and parses one vehicle type every few frames.
pub fn load_traffic_variants(
    job: Option<ResMut<WorldLoadJob>>,
    settings: Res<TrafficSettings>,
    mut pool: ResMut<TrafficPool>,
) {
    let Some(job) = job else { return };
    if settings.baseline == 0 {
        return;
    }
    if !pool.started {
        pool.started = true;
        pool.rng = Some(Rng::new(0xA11_CA45));
        let PoolSetup {
            groups,
            slots,
            defaults,
            depots,
        } = read_groups(&job);
        pool.groups = groups;
        pool.slots = slots;
        pool.defaults = defaults;
        pool.depots = depots;
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

/// The vehicles wanted per group: the baseline spread over the groups by their density-weighted
/// lane metres, each scaled by its time factor (so the total is the baseline times the average
/// factor), limited to what the roads hold ([`MIN_LANE_SPACING_M`]). The flag says the limit
/// bound.
pub(super) fn baseline_targets(
    metres: &[f64],
    factors: &[f64],
    baseline: usize,
) -> (Vec<f64>, bool) {
    let total: f64 = metres.iter().sum();
    if total <= 0.0 || baseline == 0 {
        return (vec![0.0; metres.len()], false);
    }
    let wanted: Vec<f64> = metres
        .iter()
        .zip(factors)
        .map(|(m, f)| baseline as f64 * m / total * f)
        .collect();
    let room = metres.iter().zip(factors).map(|(m, f)| m * f).sum::<f64>() / MIN_LANE_SPACING_M;
    group_targets(&wanted, room.floor().max(0.0) as usize)
}

/// The shortest road vehicle measured as such (m): a bicycle is 1.76 m long.
const MIN_LENGTH_M: f32 = 0.5;

/// A unit's length and the distance of its origin behind the front (m), from its meshes' extent
/// along the vehicle (`min_z … max_z`). Bicycles and mopeds are under 2 m (`[boundingbox]` 1.76 …
/// 1.83 m); a mesh that cannot be measured takes its `[boundingbox]` length (`boxed`), else a
/// car's.
pub(super) fn unit_size(min_z: f32, max_z: f32, boxed: Option<f32>) -> (f32, f32) {
    let len = max_z - min_z;
    if min_z.is_finite() && (MIN_LENGTH_M..25.0).contains(&len) {
        (len, (-min_z).clamp(0.0, len + 2.0))
    } else if let Some(len) = boxed.filter(|l| (MIN_LENGTH_M..25.0).contains(l)) {
        (len, len * 0.5)
    } else {
        (4.4, 2.2)
    }
}

#[cfg(test)]
mod size_tests {
    use super::unit_size;

    #[test]
    fn a_bicycle_keeps_its_length() {
        assert_eq!(unit_size(-1.0, 0.76, None), (1.76, 1.0));
        assert_eq!(
            unit_size(f32::INFINITY, f32::NEG_INFINITY, Some(1.76)),
            (1.76, 0.88)
        );
        assert_eq!(
            unit_size(-0.1, 0.1, None),
            (4.4, 2.2),
            "a speck is no vehicle"
        );
        assert_eq!(unit_size(-40.0, 0.0, Some(30.0)), (4.4, 2.2));
    }
}
