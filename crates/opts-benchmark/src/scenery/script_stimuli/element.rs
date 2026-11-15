//! Judging a channel element: the pixels of its region that change between two value classes.

use super::*;

/// Indices of the first state of each distinct value (`keys[s]` is the value in state `s`).
pub(super) fn classes(keys: &[Vec<i64>]) -> Vec<usize> {
    let mut reps: Vec<usize> = Vec::new();
    for (i, k) in keys.iter().enumerate() {
        if !reps.iter().any(|&r| keys[r] == *k) {
            reps.push(i);
        }
    }
    reps
}

pub(super) fn quantise(v: f32, step: f32) -> i64 {
    if v.is_nan() {
        i64::MIN
    } else {
        (v / step).round() as i64
    }
}

/// A channel element's region in both views and what counts as a change there.
pub(super) struct Element {
    /// Where to look.
    pub(super) regions: Vec<Mask>,
    /// The element's own area in each view (pixels); a view with fewer than
    /// `min_region_px` cannot judge it.
    pub(super) area: Vec<usize>,
    /// Changed pixels needed in each view.
    pub(super) need: Vec<usize>,
}

/// Pixels of `region` that differ between two renders: only one shows the object there, or a
/// colour channel differs by at least `delta`.
pub(super) fn changed_px(a: &Matte, b: &Matte, region: &Mask, delta: u8) -> usize {
    (0..region.px.len())
        .filter(|&i| {
            region.px[i]
                && (a.mask.px[i] || b.mask.px[i])
                && (a.mask.px[i] != b.mask.px[i]
                    || (0..3).any(|c| a.rgb[i][c].abs_diff(b.rgb[i][c]) >= delta))
        })
        .count()
}

/// A mesh element (`visible` flips, animations): its own area decides how much must change.
pub(super) fn mesh_element(j: &Judge<'_>, i: usize, dilate: usize) -> Element {
    let views = 0..j.views.len();
    let area: Vec<usize> = views.clone().map(|v| j.masks[v][i].all.count()).collect();
    Element {
        regions: views
            .map(|v| {
                if dilate == 0 {
                    j.masks[v][i].all.clone()
                } else {
                    j.masks[v][i].all.dilate(dilate)
                }
            })
            .collect(),
        need: area
            .iter()
            .map(|a| ((j.th.live_area_frac * *a as f32).ceil() as usize).max(j.th.live_min_px))
            .collect(),
        area,
    }
}

/// Whether the pixels of the element's region changed between two states in either view;
/// `None` when no view can judge it.
pub(super) fn live_pair(j: &Judge<'_>, a: usize, b: usize, e: &Element) -> Option<bool> {
    let mut judged = false;
    for (v, region) in e.regions.iter().enumerate() {
        if e.area[v] < j.th.min_region_px {
            continue;
        }
        judged = true;
        if changed_px(&j.mattes[a][v], &j.mattes[b][v], region, j.th.delta) >= e.need[v] {
            return Some(true);
        }
    }
    judged.then_some(false)
}

/// Judges one element given the representative states of its value classes. Returns `None`
/// when its region is too small in both views.
pub(super) fn element_live(j: &Judge<'_>, reps: &[usize], e: &Element) -> Option<bool> {
    let mut judged = false;
    for &r in &reps[1..] {
        match live_pair(j, reps[0], r, e) {
            Some(true) => return Some(true),
            Some(false) => judged = true,
            None => {}
        }
    }
    judged.then_some(false)
}

pub(super) fn record(t: &mut Tally, live: Option<bool>, label: String) {
    match live {
        Some(true) => {
            t.stimuli += 1;
            t.live += 1;
        }
        Some(false) => {
            t.stimuli += 1;
            t.dead.push(label);
        }
        None => t.unobservable += 1,
    }
}
