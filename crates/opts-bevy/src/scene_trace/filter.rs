//! Filters of the scene trace: by tile, by distance from a point, by kind and by file pattern.

use super::Probe;

/// A disc on the ground (Bevy x, z) things must lie in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Radius {
    /// Centre (Bevy x, z).
    pub center: [f32; 2],
    /// Radius in metres.
    pub meters: f32,
}

/// What the trace keeps. An empty filter keeps everything.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Filters {
    /// Only these tiles.
    pub tiles: Vec<(i32, i32)>,
    /// Only these kinds (see [`super::kind`]).
    pub kinds: Vec<String>,
    /// File pattern: substring, or `*` wildcards; case-insensitive.
    pub file: Option<String>,
    /// Only things inside this disc.
    pub radius: Option<Radius>,
}

impl Filters {
    /// Whether nothing is filtered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
            && self.kinds.is_empty()
            && self.file.is_none()
            && self.radius.is_none()
    }

    /// Whether the probe passes every filter.
    #[must_use]
    pub fn accepts(&self, p: &Probe<'_>) -> bool {
        if !self.tiles.is_empty() && !p.tile.is_some_and(|t| self.tiles.contains(&t)) {
            return false;
        }
        if !self.kinds.is_empty() && !self.kinds.iter().any(|k| k == p.kind) {
            return false;
        }
        if let Some(pat) = &self.file {
            if !glob_match(pat, p.file) {
                return false;
            }
        }
        if let Some(r) = &self.radius {
            let (dx, dz) = (p.world[0] - r.center[0], p.world[2] - r.center[1]);
            if dx * dx + dz * dz > r.meters * r.meters {
                return false;
            }
        }
        true
    }
}

/// Case-insensitive match: `*` matches any run; a pattern without `*` is a substring test.
#[must_use]
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern = pattern.to_lowercase().replace('\\', "/");
    let text = text.to_lowercase().replace('\\', "/");
    if !pattern.contains('*') {
        return text.contains(&pattern);
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut rest = text.as_str();
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            // Not anchored at the start: `house*.sco` finds `sceneryobjects/x/house_01.sco`.
            let Some(at) = rest.find(part) else {
                return false;
            };
            rest = &rest[at + part.len()..];
        } else if i == parts.len() - 1 {
            return rest.ends_with(part);
        } else {
            let Some(at) = rest.find(part) else {
                return false;
            };
            rest = &rest[at + part.len()..];
        }
    }
    true
}
