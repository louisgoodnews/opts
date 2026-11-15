//! Light-map channels: the light map's value changes, judged over the union of all meshes.

use super::element::*;
use super::*;

pub(super) fn lightmap_stimuli(j: &Judge<'_>, t: &mut Tally) {
    let states = &j.outcome.states;
    let count = states
        .iter()
        .map(|s| s.probe.light_maps.len())
        .max()
        .unwrap_or(0);
    if count > 0 {
        t.channels.push("lightmap");
    }
    let size = IMAGE_SIZE as usize;
    for k in 0..count {
        let keys: Vec<Vec<i64>> = states
            .iter()
            .map(|s| {
                vec![quantise(
                    s.probe.light_maps.get(k).copied().unwrap_or(f32::NAN),
                    0.05,
                )]
            })
            .collect();
        let reps = classes(&keys);
        if reps.len() < 2 {
            continue;
        }
        let regions: Vec<Mask> = (0..j.views.len())
            .map(|v| {
                let mut m = Mask::new(size, size);
                for mm in &j.masks[v] {
                    m.union_with(&mm.all);
                }
                m
            })
            .collect();
        let area: Vec<usize> = regions.iter().map(Mask::count).collect();
        let need = area
            .iter()
            .map(|a| ((j.th.live_changed_share * *a as f32).ceil() as usize).max(j.th.live_min_px))
            .collect();
        let live = element_live(
            j,
            &reps,
            &Element {
                regions,
                area,
                need,
            },
        );
        record(t, live, format!("lightmap:{k}"));
    }
}
