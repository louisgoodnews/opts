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

mod element;
mod light;
mod lightmap;
mod mesh;

use light::light_stimuli;
use lightmap::lightmap_stimuli;
use mesh::mesh_stimuli;

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
mod tests;
