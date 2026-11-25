//! The slot choice of a traffic vehicle: a group below its target with a vehicle ready, then a
//! vehicle of that group by weight.

use super::super::*;

/// Picks a group that is below its target (weighted by how far below) and has a vehicle ready,
/// then a vehicle of it by weight.
pub(in super::super) fn pick_slot(
    pool: &TrafficPool,
    deficits: &[f64],
    rng: &mut Rng,
) -> Option<usize> {
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
