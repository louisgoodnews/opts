//! Light channels: the brightness gain of a light's window between an off and an on state.

use super::*;

/// Half size of a light's brightness window (9 × 9 pixels).
const LIGHT_WINDOW: i64 = 4;

/// Mean brightness (largest channel, 0..255) of the 9 × 9 window around `(cx, cy)`.
pub(super) fn window_brightness(m: &Matte, cx: i64, cy: i64) -> f32 {
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
pub(super) fn light_gain(j: &Judge<'_>, l: usize, off: usize, on: usize) -> bool {
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

pub(super) fn light_stimuli(j: &Judge<'_>, t: &mut Tally) {
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
