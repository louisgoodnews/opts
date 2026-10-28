//! Sound configuration files (`sound.cfg`, `docs/src/formats/text/vehicles/sound_cfg.md`): a flat
//! list of `[sound]` and `[loopsound]` definitions with modifiers. Vehicles and scenery objects
//! share the format; scenery uses `[sound]` only (census 2026-10-20: 345 definitions, no
//! `[loopsound]`), with the extra keywords `[onlyone]` and `[checkloading]`.
//!
//! Rules of the spec: a keyword starts a line (`[` first, `]` last); its parameters follow one per
//! line; everything else is a comment; modifiers attach to the most recent definition. Blank
//! lines are skipped. Nothing here fails: what cannot be read is a warning.

/// How a definition plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundKind {
    /// `[sound]`.
    Sound,
    /// `[loopsound]`.
    Loop,
}

/// A volume curve: the product of all curves of a sound is its volume.
#[derive(Debug, Clone, PartialEq)]
pub struct VolCurve {
    /// The input variable.
    pub variable: String,
    /// `(input, volume)` points in file order; piecewise linear, ends held.
    pub points: Vec<(f32, f32)>,
}

impl VolCurve {
    /// The volume at `x`: linear between neighbouring points, the end values held outside, 1 for
    /// a curve without points.
    #[must_use]
    pub fn at(&self, x: f32) -> f32 {
        let p = &self.points;
        let (Some(first), Some(last)) = (p.first(), p.last()) else {
            return 1.0;
        };
        if x <= first.0 {
            return first.1;
        }
        if x >= last.0 {
            return last.1;
        }
        for w in p.windows(2) {
            let ((x0, y0), (x1, y1)) = (w[0], w[1]);
            if x >= x0 && x <= x1 {
                if (x1 - x0).abs() < f32::EPSILON {
                    return y1;
                }
                return y0 + (y1 - y0) * (x - x0) / (x1 - x0);
            }
        }
        last.1
    }
}

/// `[conditionSingle]`: a variable gate.
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    /// The variable.
    pub variable: String,
    /// The comparison value.
    pub value: f32,
    /// The third parameter (0 or 1 in the corpus; its meaning is open).
    pub mode: f32,
}

/// One sound definition with its modifiers.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundDef {
    /// `[sound]` or `[loopsound]`.
    pub kind: SoundKind,
    /// The WAV file name as written (relative to the configuration file's folder or the
    /// installation's `Sounds`).
    pub file: String,
    /// `[sound]`: the pitch factor (the second line).
    pub pitch: f32,
    /// `[loopsound]`: the sample rate line.
    pub loop_rate: Option<u32>,
    /// `[loopsound]`: the variable the pitch follows.
    pub pitch_variable: Option<String>,
    /// `[loopsound]`: the variable value at which the sample plays at its natural pitch.
    pub pitch_reference: f32,
    /// `[loopsound]`: the pitch scale.
    pub pitch_scale: f32,
    /// `[3d]`: x, y, z in the object's frame (metres, +x right, +y forward, +z up).
    pub position: Option<[f32; 3]>,
    /// `[3d]`: the range, the distance where the sound starts to fall off.
    pub range: f32,
    /// `[3d]`: the optional fifth value.
    pub extra_distance: Option<f32>,
    /// `[volcurve]`s.
    pub volcurves: Vec<VolCurve>,
    /// `[trigger]` event names.
    pub triggers: Vec<String>,
    /// `[conditionSingle]`s.
    pub conditions: Vec<Condition>,
    /// `[viewpoint]`.
    pub viewpoint: Option<i32>,
    /// `[noloop]`.
    pub noloop: bool,
    /// `[onlyone]` (scenery; meaning unverified: presumably one instance at a time).
    pub only_one: bool,
    /// `[checkloading]` (scenery; meaning unverified).
    pub check_loading: bool,
}

impl SoundDef {
    fn new(kind: SoundKind, file: String) -> Self {
        Self {
            kind,
            file,
            pitch: 1.0,
            loop_rate: None,
            pitch_variable: None,
            pitch_reference: 1.0,
            pitch_scale: 1.0,
            position: None,
            range: 1.0,
            extra_distance: None,
            volcurves: Vec::new(),
            triggers: Vec::new(),
            conditions: Vec::new(),
            viewpoint: None,
            noloop: false,
            only_one: false,
            check_loading: false,
        }
    }

    /// Whether the sound plays by itself for as long as the thing is near: no trigger and no
    /// condition start it. (Scenery `[sound]` without modifiers is a loop: the 156 definitions
    /// of that shape are hums and ambience; ❓ inferred, see the format page.)
    #[must_use]
    pub fn is_continuous(&self) -> bool {
        self.kind == SoundKind::Loop
            || (self.triggers.is_empty() && self.conditions.is_empty() && !self.noloop)
    }
}

/// A parsed file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SoundConfig {
    /// The definitions in file order.
    pub sounds: Vec<SoundDef>,
    /// What could not be read.
    pub warnings: Vec<String>,
}

fn num(s: &str) -> Option<f32> {
    s.trim().replace(',', ".").parse::<f32>().ok()
}

fn keyword(line: &str) -> Option<String> {
    let t = line.trim();
    (t.starts_with('[') && t.ends_with(']') && t.len() > 2).then(|| t.to_lowercase())
}

impl SoundConfig {
    /// Parses the text of a file (already decoded from Windows-1252).
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let lines: Vec<&str> = text.lines().map(str::trim).collect();
        let mut out = Self::default();
        let mut i = 0;
        // The next `n` non-blank lines after the keyword at `at`; stops at another keyword.
        let params = |at: usize, n: usize| -> (Vec<&str>, usize) {
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
        };
        while i < lines.len() {
            let Some(k) = keyword(lines[i]) else {
                i += 1;
                continue;
            };
            match k.as_str() {
                "[sound]" | "[loopsound]" => {
                    let is_loop = k == "[loopsound]";
                    let (p, next) = params(i, if is_loop { 5 } else { 2 });
                    if p.is_empty() {
                        out.warnings
                            .push(format!("line {}: {k} without a file", i + 1));
                        i = next.max(i + 1);
                        continue;
                    }
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
                    out.sounds.push(d);
                    i = next;
                }
                "[3d]" | "[volcurve]" | "[trigger]" | "[conditionsingle]" | "[viewpoint]"
                | "[noloop]" | "[onlyone]" | "[checkloading]" => {
                    let n = match k.as_str() {
                        "[3d]" => 4,
                        "[conditionsingle]" => 3,
                        "[volcurve]" | "[trigger]" | "[viewpoint]" => 1,
                        _ => 0,
                    };
                    let (p, mut next) = params(i, n);
                    let Some(d) = out.sounds.last_mut() else {
                        out.warnings
                            .push(format!("line {}: {k} before any sound, ignored", i + 1));
                        i = next.max(i + 1);
                        continue;
                    };
                    match k.as_str() {
                        "[3d]" => {
                            let v: Vec<f32> = p.iter().filter_map(|s| num(s)).collect();
                            if v.len() == 4 {
                                d.position = Some([v[0], v[1], v[2]]);
                                d.range = v[3].max(0.01);
                                // The optional fifth value stands after a blank line.
                                let blank_follows =
                                    lines.get(next - 1 + 1).is_some_and(|l| l.is_empty());
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
                                out.warnings
                                    .push(format!("line {}: [3d] needs four numbers", i + 1));
                            }
                        }
                        "[volcurve]" => {
                            let mut points = Vec::new();
                            let mut j = next;
                            // Points follow as `[pnt]` blocks of two numbers.
                            loop {
                                // Blank lines and prose comments may stand between the keyword
                                // and its points (`field.cfg`: "vom 31.1. bis 31.7.").
                                while j < lines.len() && keyword(lines[j]).is_none() {
                                    j += 1;
                                }
                                if lines.get(j).and_then(|l| keyword(l)).as_deref() != Some("[pnt]")
                                {
                                    break;
                                }
                                let (q, after) = params(j, 2);
                                if let (Some(x), Some(y)) = (
                                    q.first().and_then(|s| num(s)),
                                    q.get(1).and_then(|s| num(s)),
                                ) {
                                    points.push((x, y));
                                } else {
                                    out.warnings
                                        .push(format!("line {}: [pnt] needs two numbers", j + 1));
                                }
                                j = after;
                            }
                            next = j;
                            match p.first() {
                                Some(var) if !points.is_empty() => d.volcurves.push(VolCurve {
                                    variable: (*var).to_owned(),
                                    points,
                                }),
                                _ => out.warnings.push(format!(
                                    "line {}: [volcurve] without points, ignored",
                                    i + 1
                                )),
                            }
                        }
                        "[trigger]" => {
                            if let Some(t) = p.first() {
                                d.triggers.push((*t).to_owned());
                            }
                        }
                        "[conditionsingle]" => {
                            if let (Some(v), Some(x)) = (p.first(), p.get(1).and_then(|s| num(s))) {
                                d.conditions.push(Condition {
                                    variable: (*v).to_owned(),
                                    value: x,
                                    mode: p.get(2).and_then(|s| num(s)).unwrap_or(0.0),
                                });
                            } else {
                                out.warnings.push(format!(
                                    "line {}: [conditionSingle] needs three parameters",
                                    i + 1
                                ));
                            }
                        }
                        "[viewpoint]" => d.viewpoint = p.first().and_then(|s| s.parse().ok()),
                        "[noloop]" => d.noloop = true,
                        "[onlyone]" => d.only_one = true,
                        "[checkloading]" => d.check_loading = true,
                        _ => {}
                    }
                    i = next.max(i + 1);
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
}

#[cfg(test)]
mod tests;
