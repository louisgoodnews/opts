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

mod parse;

#[cfg(test)]
mod tests;

impl SoundConfig {
    /// Parses the text of a file (already decoded from Windows-1252).
    #[must_use]
    pub fn parse(text: &str) -> Self {
        parse::parse_sound_config(text)
    }
}
