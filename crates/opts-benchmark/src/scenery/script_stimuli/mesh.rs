//! Mesh channels: a `[visible]` flag and an animation that flip the mesh's own pixels.

use super::element::*;
use super::*;

/// Dilation of an animated mesh's rest-pose region (pixels).
const ANIM_REGION_PX: usize = 12;

/// Values a mesh's animation shows in a state: the runtime's shown values, or the bound
/// variables times their factors when the runtime shows none.
pub(super) fn anim_key(j: &Judge<'_>, i: usize, s: &State) -> Vec<i64> {
    let shown = s.probe.shown.get(i).filter(|v| !v.is_empty());
    match shown {
        Some(v) => v.iter().map(|x| quantise(*x, 1e-3)).collect(),
        None => j.meshes[i]
            .anim
            .iter()
            .map(|(name, f)| quantise(var(s, name) * f, 1e-3))
            .collect(),
    }
}

pub(super) fn mesh_stimuli(j: &Judge<'_>, t: &mut Tally) {
    let states = &j.outcome.states;
    for (i, m) in j.meshes.iter().enumerate().filter(|(_, m)| m.top) {
        if m.visible.is_some() {
            t.channels.push("visible");
            let keys: Vec<Vec<i64>> = states
                .iter()
                .map(|s| vec![i64::from(j.visible(i, s))])
                .collect();
            let reps = classes(&keys);
            if reps.len() > 1 {
                let live = element_live(j, &reps, &mesh_element(j, i, 0));
                record(t, live, format!("visible:{i}"));
            }
        }
        if !m.anim.is_empty() {
            t.channels.push("anim");
            let keys: Vec<Vec<i64>> = states.iter().map(|s| anim_key(j, i, s)).collect();
            let reps = classes(&keys);
            if reps.len() > 1 {
                let live = element_live(j, &reps, &mesh_element(j, i, ANIM_REGION_PX));
                record(t, live, format!("anim:{i}"));
            }
        }
    }
}
