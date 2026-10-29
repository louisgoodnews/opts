//! Parser implementation for OMSI sound configuration files (`sound.cfg`).

use super::{Condition, SoundConfig, SoundDef, SoundKind, VolCurve};

fn num(s: &str) -> Option<f32> {
    s.trim().replace(',', ".").parse::<f32>().ok()
}

fn keyword(line: &str) -> Option<String> {
    let t = line.trim();
    (t.starts_with('[') && t.ends_with(']') && t.len() > 2).then(|| t.to_lowercase())
}

fn params<'a>(lines: &[&'a str], at: usize, n: usize) -> (Vec<&'a str>, usize) {
    let mut got = Vec::new();
    let mut j = at + 1;
    while j < lines.len() && got.len() < n {
        let l = lines[j];
        if l.is_empty() {
            j += 1;
            continue;
        }
        if keyword(l).is_some() {
            break;
        }
        got.push(l);
        j += 1;
    }
    (got, j)
}

fn parse_sound_def(
    k: &str,
    p: &[&str],
    line: usize,
    warnings: &mut Vec<String>,
) -> Option<SoundDef> {
    if p.is_empty() {
        warnings.push(format!("line {}: {k} without a file", line + 1));
        return None;
    }
    let is_loop = k == "[loopsound]";
    let mut d = SoundDef::new(
        if is_loop {
            SoundKind::Loop
        } else {
            SoundKind::Sound
        },
        p[0].to_owned(),
    );
    if is_loop {
        d.loop_rate = p.get(1).and_then(|s| s.trim().parse().ok());
        d.pitch_variable = p.get(2).map(|s| (*s).to_owned());
        d.pitch_reference = p.get(3).and_then(|s| num(s)).unwrap_or(1.0);
        d.pitch_scale = p.get(4).and_then(|s| num(s)).unwrap_or(1.0);
    } else {
        d.pitch = p.get(1).and_then(|s| num(s)).unwrap_or(1.0);
    }
    Some(d)
}

fn handle_3d(
    lines: &[&str],
    p: &[&str],
    mut next: usize,
    d: &mut SoundDef,
    warnings: &mut Vec<String>,
    line: usize,
) -> usize {
    let v: Vec<f32> = p.iter().filter_map(|s| num(s)).collect();
    if v.len() == 4 {
        d.position = Some([v[0], v[1], v[2]]);
        d.range = v[3].max(0.01);
        let blank_follows = lines.get(next - 1 + 1).is_some_and(|l| l.is_empty());
        if blank_follows {
            let mut j = next;
            while j < lines.len() && lines[j].is_empty() {
                j += 1;
            }
            if let Some(x) = lines
                .get(j)
                .filter(|l| keyword(l).is_none())
                .and_then(|l| num(l))
            {
                d.extra_distance = Some(x);
                next = j + 1;
            }
        }
    } else {
        warnings.push(format!("line {}: [3d] needs four numbers", line + 1));
    }
    next
}

fn handle_volcurve(
    lines: &[&str],
    p: &[&str],
    next: usize,
    d: &mut SoundDef,
    warnings: &mut Vec<String>,
    line: usize,
) -> usize {
    let mut points = Vec::new();
    let mut j = next;
    loop {
        while j < lines.len() && keyword(lines[j]).is_none() {
            j += 1;
        }
        if lines.get(j).and_then(|l| keyword(l)).as_deref() != Some("[pnt]") {
            break;
        }
        let (q, after) = params(lines, j, 2);
        if let (Some(x), Some(y)) = (
            q.first().and_then(|s| num(s)),
            q.get(1).and_then(|s| num(s)),
        ) {
            points.push((x, y));
        } else {
            warnings.push(format!("line {}: [pnt] needs two numbers", j + 1));
        }
        j = after;
    }
    match p.first() {
        Some(var) if !points.is_empty() => d.volcurves.push(VolCurve {
            variable: (*var).to_owned(),
            points,
        }),
        _ => warnings.push(format!(
            "line {}: [volcurve] without points, ignored",
            line + 1
        )),
    }
    j
}

fn handle_condition(p: &[&str], d: &mut SoundDef, warnings: &mut Vec<String>, line: usize) {
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

fn handle_modifier_block(
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
