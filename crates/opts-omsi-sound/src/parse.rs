//! Parser implementation for OMSI sound configuration files (`sound.cfg`).

use super::{Condition, SoundConfig, SoundDef, SoundKind, VolCurve};

mod blocks;
mod conditions;
mod helpers;

use blocks::parse_sound_def;
use conditions::handle_modifier_block;
use helpers::{keyword, num, params};

pub(crate) fn parse_sound_config(text: &str) -> SoundConfig {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut out = SoundConfig::default();
    let mut i = 0;
    while i < lines.len() {
        let Some(k) = keyword(lines[i]) else {
            i += 1;
            continue;
        };
        match k.as_str() {
            "[sound]" | "[loopsound]" => {
                let is_loop = k == "[loopsound]";
                let (p, next) = params(&lines, i, if is_loop { 5 } else { 2 });
                if let Some(d) = parse_sound_def(&k, &p, i, &mut out.warnings) {
                    out.sounds.push(d);
                }
                i = next.max(i + 1);
            }
            "[3d]" | "[volcurve]" | "[trigger]" | "[conditionsingle]" | "[viewpoint]"
            | "[noloop]" | "[onlyone]" | "[checkloading]" => {
                let Some(d) = out.sounds.last_mut() else {
                    out.warnings
                        .push(format!("line {}: {k} before any sound, ignored", i + 1));
                    i += 1;
                    continue;
                };
                i = handle_modifier_block(&k, &lines, i, d, &mut out.warnings);
            }
            "[pnt]" => {
                out.warnings.push(format!(
                    "line {}: [pnt] outside a [volcurve], ignored",
                    i + 1
                ));
                i += 1;
            }
            other => {
                out.warnings
                    .push(format!("line {}: unknown keyword {other}, skipped", i + 1));
                i += 1;
            }
        }
    }
    out
}
