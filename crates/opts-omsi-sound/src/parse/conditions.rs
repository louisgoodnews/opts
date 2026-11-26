//! Conditions and modifier blocks of `sound.cfg`.

use super::blocks::{handle_3d, handle_volcurve};
use super::*;

pub(super) fn handle_condition(
    p: &[&str],
    d: &mut SoundDef,
    warnings: &mut Vec<String>,
    line: usize,
) {
    if let (Some(v), Some(x)) = (p.first(), p.get(1).and_then(|s| num(s))) {
        d.conditions.push(Condition {
            variable: (*v).to_owned(),
            value: x,
            mode: p.get(2).and_then(|s| num(s)).unwrap_or(0.0),
        });
    } else {
        warnings.push(format!(
            "line {}: [conditionSingle] needs three parameters",
            line + 1
        ));
    }
}

pub(super) fn handle_modifier_block(
    k: &str,
    lines: &[&str],
    i: usize,
    d: &mut SoundDef,
    warnings: &mut Vec<String>,
) -> usize {
    let n = match k {
        "[3d]" => 4,
        "[conditionsingle]" => 3,
        "[volcurve]" | "[trigger]" | "[viewpoint]" => 1,
        _ => 0,
    };
    let (p, next) = params(lines, i, n);
    let next_idx = match k {
        "[3d]" => handle_3d(lines, &p, next, d, warnings, i),
        "[volcurve]" => handle_volcurve(lines, &p, next, d, warnings, i),
        "[trigger]" => {
            if let Some(t) = p.first() {
                d.triggers.push((*t).to_owned());
            }
            next
        }
        "[conditionsingle]" => {
            handle_condition(&p, d, warnings, i);
            next
        }
        "[viewpoint]" => {
            d.viewpoint = p.first().and_then(|s| s.parse().ok());
            next
        }
        "[noloop]" => {
            d.noloop = true;
            next
        }
        "[onlyone]" => {
            d.only_one = true;
            next
        }
        "[checkloading]" => {
            d.check_loading = true;
            next
        }
        _ => next,
    };
    next_idx.max(i + 1)
}
