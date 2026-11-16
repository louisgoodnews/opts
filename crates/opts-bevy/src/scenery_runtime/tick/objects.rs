//! The frame's tick over all scenery objects: distance bands, the budget and the round robin.

use super::one::{due, tick_one};
use super::*;

/// Ticks the objects in `objects`. `force` ticks every object regardless of distance and
/// budget (the benchmark's deterministic step).
pub(super) fn tick_objects(
    ctx: &TickContext<'_>,
    stats: &mut SceneryRuntimeStats,
    objects: &mut ObjectQuery,
    targets: &mut EffectTargets,
) {
    let started = Instant::now();
    let total = objects.iter().len();
    let start = if total == 0 { 0 } else { stats.cursor % total };
    let (mut ticks, mut script_ticks, mut scripted, mut last) = (0, 0, 0, start);
    // Two passes from the round-robin cursor so the same objects do not always go first.
    for pass in 0..2 {
        for (i, (mut rt, scripts, transform)) in objects.iter_mut().enumerate() {
            if (pass == 0) != (i >= start) {
                continue;
            }
            rt.pending_s += ctx.dt;
            if scripts.is_some() {
                scripted += 1;
            }
            let interval = match ctx.camera {
                _ if ctx.force => 0.0,
                None => ctx.settings.mid_interval_s,
                Some(cam) => {
                    let d = transform.translation().distance(cam);
                    if d <= ctx.settings.near_m {
                        0.0
                    } else if d <= ctx.settings.mid_m {
                        ctx.settings.mid_interval_s
                    } else {
                        ctx.settings.far_interval_s
                    }
                }
            };
            if !due(
                &rt,
                scripts.is_some(),
                ctx.host,
                interval,
                ctx.force,
                ctx.paused,
            ) {
                continue;
            }
            if !ctx.force && ticks >= ctx.settings.max_ticks_per_frame {
                continue;
            }
            ticks += 1;
            last = i + 1;
            let ran = tick_one(
                &mut rt,
                scripts.map(Mut::into_inner),
                ctx.host,
                ctx.overrides,
                targets,
                stats,
            );
            script_ticks += usize::from(ran);
        }
    }
    stats.cursor = if ticks >= ctx.settings.max_ticks_per_frame {
        last
    } else {
        0
    };
    stats.objects = total;
    stats.scripted = scripted;
    stats.ticks = ticks;
    stats.script_ticks = script_ticks;
    stats.tick_time_us = started.elapsed().as_secs_f32() * 1e6;
}
