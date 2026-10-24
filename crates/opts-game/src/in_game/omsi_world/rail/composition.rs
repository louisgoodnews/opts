//! The units of a composed rail vehicle as the game keeps them.

use bevy::prelude::*;

use super::super::BodyBox;

/// One sub part of a composed vehicle (a car, a bogie, an articulation joint, a coupler).
#[derive(Debug, Clone)]
pub struct Part {
    /// The entity carrying the part's meshes and transform (the root for the head part).
    pub entity: Entity,
    /// Name of the part (its `.bus` file name; the head: the vehicle's model name).
    pub name: String,
    /// Distance of the part's origin behind the head's origin along the vehicle axis (m).
    pub behind: f32,
    /// Half distance between the part's bogies (`[boogies]`), 0 when unknown.
    pub half_span: f32,
    /// The part hangs backwards (coupled with the `true` flag).
    pub reversed: bool,
    /// Meshes spawned for the part (0 for the head, which is counted with the vehicle).
    pub meshes: usize,
    /// Vertices of the part's meshes.
    pub vertices: usize,
    /// The part's footprint for collisions (`[boundingbox]` of its `.bus`).
    pub body: Option<BodyBox>,
}

/// The sub parts of a composed vehicle, resolved once at spawn: the head and the coupled units
/// behind it in order. Lookups by name or index need no `[couple_back]` walk or file access.
#[derive(Component, Debug, Clone)]
pub struct VehicleComposition {
    /// The leading unit (the vehicle's root entity).
    pub head: Part,
    /// The coupled units behind the head, front to back.
    pub followers: Vec<Part>,
}

impl VehicleComposition {
    /// All parts, head first.
    pub fn parts(&self) -> impl Iterator<Item = &Part> {
        std::iter::once(&self.head).chain(self.followers.iter())
    }

    /// The first part with this name (case-insensitive, with or without `.bus`/`.ovh`).
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&Part> {
        let bare = |n: &str| {
            n.trim_end_matches(".bus")
                .trim_end_matches(".ovh")
                .to_string()
        };
        let want = bare(name);
        self.parts()
            .find(|p| bare(&p.name).eq_ignore_ascii_case(&want))
    }

    /// Number of parts including the head.
    #[must_use]
    pub fn len(&self) -> usize {
        1 + self.followers.len()
    }

    /// Always false: a composition has a head.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Distance from the head's origin to the origin of the last part (m).
    #[must_use]
    pub fn length(&self) -> f32 {
        self.followers.last().map_or(0.0, |p| p.behind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn part(name: &str, behind: f32) -> Part {
        Part {
            entity: Entity::PLACEHOLDER,
            name: name.into(),
            behind,
            half_span: 1.0,
            reversed: false,
            meshes: 1,
            vertices: 3,
            body: None,
        }
    }

    #[test]
    fn composition_finds_parts_by_name_and_reports_length() {
        let c = VehicleComposition {
            head: part("NF6D", 0.0),
            followers: vec![part("NF6D_C.bus", 12.5), part("NF6D_B_User.bus", 30.0)],
        };
        assert_eq!(c.len(), 3);
        assert!((c.length() - 30.0).abs() < 1e-6);
        assert_eq!(c.find("nf6d_c").map(|p| p.behind), Some(12.5));
        assert!(c.find("missing").is_none());
        assert_eq!(c.parts().count(), 3);
    }
}
