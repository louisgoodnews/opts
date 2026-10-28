//! Consistency and determinism checks for the script-effect judgement.

use super::*;

/// Where the runtime draws light sprites in a state (pixels that say nothing about meshes).
fn sprite_discs(j: &Judge<'_>, si: usize, v: usize) -> Mask {
    let state = &j.outcome.states[si];
    let mut discs = Mask::new(IMAGE_SIZE as usize, IMAGE_SIZE as usize);
    for l in &state.probe.lights {
        if !l.shown {
            continue;
        }
        let view = &j.views[v];
        if let Some(c) = view.project(l.pos) {
            let edge = view
                .project(l.pos + bevy::math::Vec3::X * l.size_m)
                .unwrap_or(c);
            // `size_m` is the sprite's diameter.
            discs.add_disc(c, (edge - c).length() * 0.6 + 3.0);
        }
    }
    discs
}

/// Check 1: the rendered silhouette follows the `[visible]` rules.
pub(super) fn consistency(j: &Judge<'_>, out: &mut Judgement) {
    let th = j.th;
    let has_anim = j.has_anim();
    for (si, state) in j.outcome.states.iter().enumerate() {
        if state.name == "s0b" {
            continue;
        }
        let shown = |i: usize| j.meshes[i].top && j.visible(i, state);
        let fixed = |i: usize| shown(i) && j.meshes[i].anim.is_empty();
        for v in 0..j.views.len() {
            let discs = sprite_discs(j, si, v);
            let r = j.mattes[si][v].mask.minus(&discs);
            let vis_all = j.union(v, false, fixed).minus(&discs);
            let vis_solid = j
                .union(v, true, |i| fixed(i) && j.meshes[i].visible.is_none())
                .minus(&discs);
            let moving = j
                .union(v, false, |i| {
                    j.meshes[i].top && !j.meshes[i].anim.is_empty()
                })
                .dilate(12);
            let mut note = |text: String| {
                if out.check.mismatches.len() < MAX_MISMATCHES {
                    out.check
                        .mismatches
                        .push(format!("{} v{} {text}", state.name, v + 1));
                }
            };
            // Only meshes without a `[visible]` rule: a rule-driven mesh may sit behind others.
            let n = vis_solid.count();
            let alpha = vis_all.minus_count(&vis_solid) as f32 / vis_all.count().max(1) as f32;
            if n >= 200 && alpha <= 0.5 {
                let recall = r.dilate(3).and_count(&vis_solid) as f32 / n as f32;
                if recall < 0.6 {
                    note(format!(
                        "missing (recall {recall:.2}, expected {n} px, rendered {} of {} px)",
                        r.count(),
                        j.mattes[si][v].mask.count()
                    ));
                }
            }
            if !has_anim && r.count() >= 50 && vis_all.count() > 0 {
                let precision = r.and_count(&vis_all.dilate(3)) as f32 / r.count() as f32;
                if precision < 0.90 {
                    note(format!("outside (precision {precision:.2})"));
                }
            }
            let covered = {
                let mut c = vis_all.clone();
                c.union_with(&moving);
                c.dilate(2)
            };
            for i in 0..j.meshes.len() {
                let hidden = j.meshes[i].top && j.meshes[i].anim.is_empty() && !j.visible(i, state);
                if !hidden {
                    continue;
                }
                let own = j.masks[v][i].all.minus(&covered).minus(&discs);
                if own.count() >= th.leak_min_px
                    && r.and_count(&own) as f32 / own.count() as f32 >= th.leak_share
                {
                    note(format!("hidden mesh {i} is drawn"));
                }
            }
        }
    }
    if !out.check.mismatches.is_empty() {
        add(out, "S-RENDER-MISMATCH");
    }
}

/// Check 6: the two runs of S0 agree.
pub(super) fn determinism(j: &Judge<'_>, out: &mut Judgement) {
    let find = |n: &str| j.outcome.states.iter().find(|s| s.name == n);
    let (Some(a), Some(b)) = (find("s0"), find("s0b")) else {
        return;
    };
    let mut diff = 0usize;
    let mut total = 0usize;
    for v in 0..a.pixels.len().min(b.pixels.len()) {
        for (pa, pb) in a.pixels[v]
            .black
            .chunks_exact(4)
            .zip(b.pixels[v].black.chunks_exact(4))
        {
            total += 1;
            diff += usize::from(pa != pb);
        }
    }
    let share = diff as f32 / total.max(1) as f32;
    out.check.nondeterministic = Some(share);
    out.check.nondeterministic_vars = a
        .probe
        .vars
        .iter()
        .filter(|(k, v)| (b.probe.vars.get(*k).copied().unwrap_or(0.0) - **v).abs() > 1e-4)
        .map(|(k, v)| format!("{k}={v}/{}", b.probe.vars.get(k).copied().unwrap_or(0.0)))
        .take(10)
        .collect();
    if share > j.th.det_max_diff {
        add(out, "S-NONDETERMINISTIC");
    }
}
