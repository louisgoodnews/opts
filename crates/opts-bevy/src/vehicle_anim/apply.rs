//! Applying the steps to a mesh: the animation matrix from the values, and the suspension rest.

use super::*;

impl MeshAnimation {
    /// Builds the rules of `mesh`; `None` when it has neither animation nor visibility rules.
    /// `pivot` is the mesh's own origin in OMSI vehicle axes (the matrix of its `.o3d`), used by
    /// animations declared `origin_from_mesh`.
    #[must_use]
    pub fn from_mesh(owner: Entity, mesh: &ModelMesh, pivot: Option<MeshPivot>) -> Option<Self> {
        if mesh.anims.iter().any(|a| needs_matrix(a, pivot)) {
            warn_matrix_once(&mesh.path);
        }
        let steps: Vec<AnimStep> = mesh.anims.iter().flat_map(|a| steps_of(a, pivot)).collect();
        let visible: Vec<(String, f32)> = mesh
            .visibility
            .iter()
            .map(|v| (v.variable.clone(), v.expected_value))
            .collect();
        (!steps.is_empty() || !visible.is_empty()).then_some(Self {
            owner,
            steps,
            visible,
            shown: Vec::new(),
        })
    }

    /// Moves the shown values towards `targets` (one per step) over `dt` seconds: first damped by
    /// each step's `delay` (exponential approach), then limited by its `maxspeed`; the first call
    /// adopts the targets.
    pub fn follow(&mut self, targets: &[f32], dt: f32) {
        if self.shown.len() != targets.len() {
            self.shown = targets.to_vec();
            return;
        }
        for ((shown, target), step) in self.shown.iter_mut().zip(targets).zip(&self.steps) {
            *shown = step.follow(*shown, *target, dt);
        }
    }
}

impl AnimStep {
    /// The value shown after `dt` seconds of following `target` from `shown`.
    #[must_use]
    pub fn follow(&self, shown: f32, target: f32, dt: f32) -> f32 {
        let mut delta = target - shown;
        if let Some(rate) = self.delay {
            delta *= 1.0 - (-rate * dt.max(0.0)).exp();
        }
        if let Some(speed) = self.maxspeed {
            let max = speed * dt;
            delta = delta.clamp(-max, max);
        }
        shown + delta
    }
}

/// The mesh transform for the current variable values, in Bevy axes (the mesh vertices are
/// already converted to Bevy axes, so the OMSI-space motion is conjugated with the basis change).
#[must_use]
pub fn animation_matrix(steps: &[AnimStep], value: impl Fn(&str) -> f32) -> Mat4 {
    let values: Vec<f32> = steps.iter().map(|s| value(&s.variable)).collect();
    animation_matrix_from_values(steps, &values)
}

/// Like [`animation_matrix`] with the variable value of each step given directly.
#[must_use]
pub fn animation_matrix_from_values(steps: &[AnimStep], values: &[f32]) -> Mat4 {
    let mut m = Mat4::IDENTITY;
    for (s, value) in steps.iter().zip(values) {
        let v = value * s.factor;
        let a = if s.translate {
            Mat4::from_translation(Vec3::X * v)
        } else {
            Mat4::from_rotation_x(-v.to_radians())
        };
        m = s.frame * a * s.frame.inverse() * m;
    }
    let c = omsi_to_bevy_basis();
    c * m * c.inverse()
}

/// The rest value of an `Axle_Suspension_<n>_<l|r>` variable (m; 0 for any other name and for a
/// vehicle without a body model): OMSI counts the travel from the unloaded spring, so a wheel
/// standing still reads about -0.1 m, while the models draw the wheels where they stand at rest
/// (the levelling's set point is the rest) ❓. The animation shows the travel from rest.
pub(super) fn suspension_rest(
    body: Option<&opts_sim::vehicle::suspension::BodySuspension>,
    name: &str,
) -> f32 {
    let Some(body) = body else {
        return 0.0;
    };
    let Some(rest) = name
        .to_ascii_lowercase()
        .strip_prefix("axle_suspension_")
        .map(|r| {
            let mut parts = r.split('_');
            let axle = parts.next().and_then(|a| a.parse::<usize>().ok());
            let side = match parts.next() {
                Some("r") => 1,
                _ => 0,
            };
            axle.and_then(|a| body.wheels.get(a * 2 + side))
                .map_or_else(
                    || body.rest_travel_m(),
                    |w| w.omsi_travel_m() + w.compression_m,
                )
        })
    else {
        return 0.0;
    };
    rest
}
