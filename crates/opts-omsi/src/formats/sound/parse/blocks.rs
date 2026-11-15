//! Sound definitions and the 3D and volume curve blocks of `sound.cfg`.

use super::*;

pub(super) fn parse_sound_def(
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

pub(super) fn handle_3d(
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

pub(super) fn handle_volcurve(
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
