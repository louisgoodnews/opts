//! Name interning, variable stores and constant tables of the script engine.

use std::collections::HashMap;

/// Case-insensitive name → dense index. OMSI matches variable, macro and constant names
/// case-insensitively (`osc.md`).
#[derive(Debug, Clone, Default)]
pub struct Interner {
    ids: HashMap<String, u32>,
    names: Vec<String>,
}

impl Interner {
    /// Index of `name`, assigning a new one on first use.
    pub fn intern(&mut self, name: &str) -> u32 {
        let key = name.to_lowercase();
        if let Some(id) = self.ids.get(&key) {
            return *id;
        }
        let id = self.names.len() as u32;
        self.names.push(key.clone());
        self.ids.insert(key, id);
        id
    }

    /// Index of `name` if it was interned before.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<u32> {
        self.ids.get(&name.to_lowercase()).copied()
    }

    /// The (lower-case) name behind an index.
    #[must_use]
    pub fn name(&self, id: u32) -> &str {
        self.names.get(id as usize).map_or("?", String::as_str)
    }

    /// Number of interned names.
    #[must_use]
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether nothing was interned yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// Float and string variables of one vehicle (shared by all of its scripts), indexed by the
/// interned name. Unwritten variables read as `0` / `""`.
#[derive(Debug, Clone, Default)]
pub struct VariableStore {
    floats: Vec<f32>,
    strings: Vec<String>,
}

impl VariableStore {
    /// Reads float variable `id`.
    #[must_use]
    pub fn float(&self, id: u32) -> f32 {
        self.floats.get(id as usize).copied().unwrap_or(0.0)
    }

    /// Writes float variable `id`.
    pub fn set_float(&mut self, id: u32, value: f32) {
        let i = id as usize;
        if self.floats.len() <= i {
            self.floats.resize(i + 1, 0.0);
        }
        self.floats[i] = value;
    }

    /// Reads string variable `id`.
    #[must_use]
    pub fn string(&self, id: u32) -> &str {
        self.strings.get(id as usize).map_or("", String::as_str)
    }

    /// Writes string variable `id`.
    pub fn set_string(&mut self, id: u32, value: String) {
        let i = id as usize;
        if self.strings.len() <= i {
            self.strings.resize(i + 1, String::new());
        }
        self.strings[i] = value;
    }
}

/// A one-dimensional piecewise-linear curve (`[newcurve]` of a constant file); clamped outside
/// its first and last point.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Curve {
    /// `(x, y)` points sorted by `x`.
    pub points: Vec<(f32, f32)>,
}

impl Curve {
    /// Builds a curve; points are sorted by `x`.
    #[must_use]
    pub fn new(mut points: Vec<(f32, f32)>) -> Self {
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        Self { points }
    }

    /// Value of the curve at `x`.
    #[must_use]
    pub fn eval(&self, x: f32) -> f32 {
        let (Some(first), Some(last)) = (self.points.first(), self.points.last()) else {
            return 0.0;
        };
        if x <= first.0 {
            return first.1;
        }
        if x >= last.0 {
            return last.1;
        }
        let i = self.points.partition_point(|p| p.0 <= x);
        let (a, b) = (self.points[i - 1], self.points[i]);
        let span = b.0 - a.0;
        if span <= f32::EPSILON {
            b.1
        } else {
            a.1 + (b.1 - a.1) * (x - a.0) / span
        }
    }
}

/// Scalar constants and curves read with `(C.L.x)` and `(F.L.x)`.
#[derive(Debug, Clone, Default)]
pub struct ConstTable {
    scalars: HashMap<String, f32>,
    curves: HashMap<String, Curve>,
}

impl ConstTable {
    /// Defines (or replaces) a scalar constant.
    pub fn set_scalar(&mut self, name: &str, value: f32) {
        self.scalars.insert(name.to_lowercase(), value);
    }

    /// Defines (or replaces) a curve.
    pub fn set_curve(&mut self, name: &str, curve: Curve) {
        self.curves.insert(name.to_lowercase(), curve);
    }

    /// Scalar constant `name`.
    #[must_use]
    pub fn scalar(&self, name: &str) -> Option<f32> {
        self.scalars.get(&name.to_lowercase()).copied()
    }

    /// Curve `name`.
    #[must_use]
    pub fn curve(&self, name: &str) -> Option<&Curve> {
        self.curves.get(&name.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interner_is_case_insensitive() {
        let mut i = Interner::default();
        let a = i.intern("GetTime");
        assert_eq!(i.intern("gettime"), a);
        assert_ne!(i.intern("other"), a);
        assert_eq!(i.get("GETTIME"), Some(a));
        assert_eq!(i.name(a), "gettime");
    }

    #[test]
    fn unwritten_variables_read_zero_and_empty() {
        let mut v = VariableStore::default();
        assert_eq!(v.float(5), 0.0);
        v.set_float(5, 2.5);
        v.set_string(2, "ab".into());
        assert_eq!((v.float(5), v.string(2), v.string(9)), (2.5, "ab", ""));
    }

    #[test]
    fn curve_interpolates_and_clamps() {
        let c = Curve::new(vec![(10.0, 100.0), (0.0, 0.0)]);
        assert_eq!(c.eval(-5.0), 0.0);
        assert_eq!(c.eval(5.0), 50.0);
        assert_eq!(c.eval(99.0), 100.0);
        assert_eq!(Curve::default().eval(1.0), 0.0);
    }
}
