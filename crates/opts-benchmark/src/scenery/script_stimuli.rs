//! Liveness and light checks of the script-effect check: for every channel element the script
//! drives (a `[visible]` mesh, an animated mesh, a light map, a light), find two captured states
//! in which its bound value differs and test that the pixels of its region changed
//! (`docs/src/architecture/scenery-benchmark.md`, Script-effect check, checks 2 and 3).

use bevy::math::Vec3;

use super::framing::IMAGE_SIZE;
use super::metrics::Matte;
use super::raster::Mask;
use super::script_judge::{var, Judge, Judgement};
use super::script_run::State;

/// Dilation of an animated mesh's rest-pose region (pixels).
const ANIM_REGION_PX: usize = 12;
/// Half size of a light's brightness window (9 × 9 pixels).
const LIGHT_WINDOW: i64 = 4;

/// Running counts of the stimuli.
#[derive(Default)]
struct Tally {
    stimuli: usize,
    live: usize,
    light_stimuli: usize,
    light_live: usize,
    dead: Vec<String>,
    /// Elements whose value changes but whose effect cannot be seen from the views.
    unobservable: usize,
    /// Channel elements bound at all.
    channels: Vec<&'static str>,
}

/// Indices of the first state of each distinct value (`keys[s]` is the value in state `s`).
fn classes(keys: &[Vec<i64>]) -> Vec<usize> {
    let mut reps: Vec<usize> = Vec::new();
    for (i, k) in keys.iter().enumerate() {
        if !reps.iter().any(|&r| keys[r] == *k) {
            reps.push(i);
        }
    }
    reps
}

fn quantise(v: f32, step: f32) -> i64 {
    if v.is_nan() {
        i64::MIN
    } else {
        (v / step).round() as i64
    }
}

/// A channel element's region in both views and what counts as a change there.
struct Element {
    /// Where to look.
    regions: Vec<Mask>,
    /// The element's own area in each view (pixels); a view with fewer than
    /// `min_region_px` cannot judge it.
    area: Vec<usize>,
    /// Changed pixels needed in each view.
    need: Vec<usize>,
}

/// Pixels of `region` that differ between two renders: only one shows the object there, or a
/// colour channel differs by at least `delta`.
fn changed_px(a: &Matte, b: &Matte, region: &Mask, delta: u8) -> usize {
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
fn mesh_element(j: &Judge<'_>, i: usize, dilate: usize) -> Element {
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
fn live_pair(j: &Judge<'_>, a: usize, b: usize, e: &Element) -> Option<bool> {
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
fn element_live(j: &Judge<'_>, reps: &[usize], e: &Element) -> Option<bool> {
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

fn record(t: &mut Tally, live: Option<bool>, label: String) {
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

/// Values a mesh's animation shows in a state: the runtime's shown values, or the bound
/// variables times their factors when the runtime shows none.
fn anim_key(j: &Judge<'_>, i: usize, s: &State) -> Vec<i64> {
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

fn mesh_stimuli(j: &Judge<'_>, t: &mut Tally) {
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

fn lightmap_stimuli(j: &Judge<'_>, t: &mut Tally) {
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

/// Mean brightness (largest channel, 0..255) of the 9 × 9 window around `(cx, cy)`.
fn window_brightness(m: &Matte, cx: i64, cy: i64) -> f32 {
    let n = IMAGE_SIZE as i64;
    let (mut sum, mut count) = (0.0f32, 0usize);
    for y in (cy - LIGHT_WINDOW).max(0)..=(cy + LIGHT_WINDOW).min(n - 1) {
        for x in (cx - LIGHT_WINDOW).max(0)..=(cx + LIGHT_WINDOW).min(n - 1) {
            let i = (y * n + x) as usize;
            let rgb = if m.mask.px[i] { m.rgb[i] } else { [0, 0, 0] };
            sum += f32::from(rgb.into_iter().max().unwrap_or(0));
            count += 1;
        }
    }
    sum / count.max(1) as f32
}

/// Brightness gain of light `l` between two states, best of both views.
fn light_gain(j: &Judge<'_>, l: usize, off: usize, on: usize) -> bool {
    let pos: Vec3 = j.outcome.states[on].probe.lights[l].pos;
    (0..j.views.len()).any(|v| {
        let Some(p) = j.views[v].project(pos) else {
            return false;
        };
        let (cx, cy) = (p.x as i64, p.y as i64);
        let a = window_brightness(&j.mattes[off][v], cx, cy);
        let b = window_brightness(&j.mattes[on][v], cx, cy);
        b - a >= (a * j.th.light_gain).max(j.th.light_gain_abs)
    })
}

fn light_stimuli(j: &Judge<'_>, t: &mut Tally) {
    let states = &j.outcome.states;
    if !j.sco.lights.is_empty() {
        t.channels.push("light");
    }
    for (l, light) in j.sco.lights.iter().enumerate() {
        let expected = |s: &State| -> f32 {
            let v = if light.variable.trim().is_empty() {
                1.0
            } else {
                var(s, light.variable.trim())
            };
            (v * light.factor).clamp(0.0, 1.0)
        };
        let on: Vec<usize> = (0..states.len())
            .filter(|&s| expected(&states[s]) > 0.01)
            .collect();
        let off = (0..states.len()).find(|&s| expected(&states[s]) <= 0.01);
        let (Some(off), false) = (off, on.is_empty()) else {
            continue;
        };
        let applied = |s: usize| states[s].probe.lights.get(l);
        if on.iter().any(|&s| applied(s).is_none()) {
            t.unobservable += 1;
            continue;
        }
        let lit = on
            .iter()
            .copied()
            .find(|&s| applied(s).is_some_and(|p| p.level > 0.01));
        let Some(lit) = lit else {
            // The variable says on, the runtime applied nothing.
            t.light_stimuli += 1;
            t.dead.push(format!("light:{l}"));
            continue;
        };
        let shown = on
            .iter()
            .copied()
            .find(|&s| applied(s).is_some_and(|p| p.level > 0.01 && p.shown));
        let Some(shown) = shown else {
            // Outside the cone or the draw distance of the first capture camera.
            let _ = lit;
            t.unobservable += 1;
            continue;
        };
        t.light_stimuli += 1;
        if light_gain(j, l, off, shown) {
            t.light_live += 1;
        } else {
            t.dead.push(format!("light:{l}"));
        }
    }
}

fn failing(stimuli: usize, live: usize, fraction: f32) -> bool {
    stimuli > 0 && (live as f32) < fraction * stimuli as f32
}

/// Checks 2 and 3 plus the `S-INERT` / `no-channel` bookkeeping.
pub fn judge(j: &Judge<'_>, out: &mut Judgement) {
    let mut t = Tally::default();
    if !j.masks.is_empty() {
        mesh_stimuli(j, &mut t);
        lightmap_stimuli(j, &mut t);
    }
    light_stimuli(j, &mut t);
    t.channels.sort_unstable();
    t.channels.dedup();
    let c = &mut out.check;
    c.status = "ok".into();
    c.channels = t.channels.iter().map(ToString::to_string).collect();
    c.stimuli = t.stimuli;
    c.live = t.live;
    c.light_stimuli = t.light_stimuli;
    c.light_live = t.light_live;
    c.dead = t.dead;
    if failing(t.stimuli, t.live, j.th.live_fraction) {
        out.categories.push("S-EFFECT-NOT-SHOWN".into());
    }
    if failing(t.light_stimuli, t.light_live, j.th.live_fraction) {
        out.categories.push("S-LIGHT-NOT-SHOWN".into());
    }
    if t.stimuli + t.light_stimuli == 0 {
        if t.unobservable > 0 {
            c.status = "unobservable".into();
        } else if c.channels.is_empty() {
            c.status = "no-channel".into();
        } else {
            c.status = "S-INERT".into();
            out.inert = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes_pick_the_first_state_of_each_value() {
        let keys = vec![vec![0], vec![0], vec![1], vec![0], vec![2], vec![1]];
        assert_eq!(classes(&keys), vec![0, 2, 4]);
        assert_eq!(classes(&[vec![5], vec![5]]), vec![0]);
    }

    #[test]
    fn quantising_groups_close_values_and_keeps_nan_apart() {
        assert_eq!(quantise(0.501, 0.05), quantise(0.499, 0.05));
        assert_ne!(quantise(0.5, 0.05), quantise(0.6, 0.05));
        assert_eq!(quantise(f32::NAN, 0.05), i64::MIN);
    }

    #[test]
    fn an_object_fails_when_fewer_than_half_of_its_stimuli_are_live() {
        assert!(!failing(0, 0, 0.5));
        assert!(failing(4, 1, 0.5));
        assert!(!failing(4, 2, 0.5));
        assert!(failing(1, 0, 0.5));
    }
}
