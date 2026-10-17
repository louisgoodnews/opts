//! `[LOD]` selection for scenery objects: exclusive bands of screen size
//! (`docs/src/architecture/scenery-runtime.md`, "Level of detail").

use bevy::prelude::*;

/// How LOD groups are chosen.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SceneryLodMode {
    /// By the object's size on screen as seen from the active 3D camera.
    #[default]
    Camera,
    /// Always the finest group (largest threshold); deterministic for benchmarks.
    Finest,
    /// Every group at once (the behaviour before LOD support).
    All,
}

/// LOD groups of one object (on its root).
#[derive(Component, Debug, Clone)]
pub struct SceneryLod {
    /// Group entities with their `[LOD]` threshold.
    pub groups: Vec<(Entity, f32)>,
    /// Bounding-sphere centre in the object's frame.
    pub centre: Vec3,
    /// Bounding-sphere radius (m).
    pub radius: f32,
    /// Threshold of the group shown last (`None`: none, `Some(NaN)`: not decided yet).
    pub shown: Option<f32>,
}

impl SceneryLod {
    /// Creates the LOD state; nothing is decided until the first update.
    #[must_use]
    pub fn new(groups: Vec<(Entity, f32)>, centre: Vec3, radius: f32) -> Self {
        Self {
            groups,
            centre,
            radius,
            shown: Some(f32::NAN),
        }
    }
}

/// The threshold of the band that contains screen size `s`: the largest threshold not above
/// `s`; `None` when `s` is below every threshold (only the always-drawn meshes show).
#[must_use]
pub fn select_band(thresholds: impl IntoIterator<Item = f32>, s: f32) -> Option<f32> {
    thresholds
        .into_iter()
        .filter(|t| *t <= s)
        .fold(None, |best: Option<f32>, t| {
            Some(best.map_or(t, |b| b.max(t)))
        })
}

/// Screen size of a sphere: radius over the half height of the view at its distance.
#[must_use]
pub fn screen_size(radius: f32, distance: f32, fov_y_rad: f32) -> f32 {
    let half = distance.max(1e-3) * (fov_y_rad * 0.5).tan();
    radius / half.max(1e-6)
}

/// Shows the LOD group of each object that matches its current screen size.
pub fn scenery_lod_system(
    mode: Res<SceneryLodMode>,
    cameras: Query<(&Camera, &GlobalTransform, &Projection), With<crate::MainCamera>>,
    mut objects: Query<(&mut SceneryLod, &GlobalTransform)>,
    mut groups: Query<&mut Visibility>,
) {
    let camera = cameras
        .iter()
        .filter(|(c, _, _)| c.is_active)
        .map(|(_, t, p)| {
            let fov = match p {
                Projection::Perspective(p) => p.fov,
                _ => std::f32::consts::FRAC_PI_4,
            };
            (t.translation(), fov)
        })
        .next();
    for (mut lod, transform) in &mut objects {
        let chosen = match *mode {
            SceneryLodMode::All => None,
            SceneryLodMode::Finest => lod.groups.iter().map(|g| g.1).reduce(f32::max),
            SceneryLodMode::Camera => {
                let Some((cam, fov)) = camera else { continue };
                let d = transform.transform_point(lod.centre).distance(cam);
                select_band(
                    lod.groups.iter().map(|g| g.1),
                    screen_size(lod.radius, d, fov),
                )
            }
        };
        let all = *mode == SceneryLodMode::All;
        let same = match (lod.shown, chosen) {
            (Some(a), Some(b)) => a == b,
            (None, None) => !all,
            _ => false,
        };
        if same {
            continue;
        }
        lod.shown = if all { Some(f32::NAN) } else { chosen };
        for &(e, t) in &lod.groups {
            let want = if all || Some(t) == chosen {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if let Ok(mut v) = groups.get_mut(e) {
                if *v != want {
                    *v = want;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_are_exclusive_and_zero_is_the_stand_in() {
        let t = [0.15, 0.0];
        assert_eq!(select_band(t, 0.5), Some(0.15));
        assert_eq!(select_band(t, 0.15), Some(0.15));
        assert_eq!(select_band(t, 0.1), Some(0.0));
        assert_eq!(select_band(t, 0.0), Some(0.0));
        // A lone threshold: nothing below it.
        assert_eq!(select_band([0.3], 0.2), None);
        assert_eq!(select_band([0.3], 0.4), Some(0.3));
    }

    #[test]
    fn screen_size_halves_with_twice_the_distance() {
        let fov = 60f32.to_radians();
        let near = screen_size(2.0, 10.0, fov);
        let far = screen_size(2.0, 20.0, fov);
        assert!((near / far - 2.0).abs() < 1e-4);
        // A 2 m radius at 10 m with a 60° view: 2 / (10 · tan 30°) ≈ 0.35.
        assert!((near - 0.346).abs() < 0.01, "{near}");
    }
}
