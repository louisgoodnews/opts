//! Script-visible effects of a scenery object: `[visible]`/`[newanim]` mesh rules, light
//! coronas and light maps, updated from the object's variables after each tick.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_sim::script::ScriptEngine;
use opts_traffic::SignalProgram;

use super::clock::{HostVar, VarRef};
use super::scripts::HostValues;
use crate::vehicle_anim::{animation_matrix_from_values, MeshAnimation};
use crate::vehicle_lights::MAX_BRIGHTNESS;

/// Runtime state of a placed scenery object that has rules, lights, LOD or a script (on the
/// object's root entity).
#[derive(Component, Debug, Clone, Default)]
pub struct SceneryObjectRuntime {
    /// Mesh entities with a [`SceneryMeshRule`].
    pub meshes: Vec<Entity>,
    /// Light entities with a [`SceneryLight`].
    pub lights: Vec<Entity>,
    /// Part entities with a [`SceneryLightMap`].
    pub light_maps: Vec<Entity>,
    /// The light program that sets `TrafficLightPhase` (`[varparent]` junction or own program).
    pub signal: Option<SignalProgram>,
    /// Seconds not yet passed to the script (ticks are skipped by distance and budget).
    pub pending_s: f32,
    /// Night switch value at the last update (objects without a script update when it flips).
    pub last_night: f32,
    /// Effects were applied at least once.
    pub updated: bool,
}

impl SceneryObjectRuntime {
    /// `TrafficLightPhase` now, if the object has a light program.
    #[must_use]
    pub fn phase(&self, host: &HostValues) -> Option<f32> {
        self.signal
            .as_ref()
            .map(|p| p.state_at(host.program_clock_s) as f32)
    }
}

/// `[visible]` and `[newanim]` rules of one mesh of a scenery object.
#[derive(Component, Debug, Clone)]
pub struct SceneryMeshRule {
    /// Steps, `[visible]` rules and shown values (the vehicle machinery; `owner` is the object
    /// root).
    pub anim: MeshAnimation,
    /// Variable of each step, then of each `[visible]` rule.
    pub vars: Vec<VarRef>,
}

impl SceneryMeshRule {
    /// Whether the mesh is shown for these values (one per `[visible]` rule).
    #[must_use]
    pub fn visible_for(&self, values: &[f32]) -> bool {
        self.anim
            .visible
            .iter()
            .zip(values)
            .all(|((_, expected), v)| (v - expected).abs() < 1e-3)
    }
}

/// A light effect (`[light_enh]`, `[light_enh_2]`) drawn as a glowing sprite.
#[derive(Component, Debug, Clone)]
pub struct SceneryLight {
    /// Brightness variable.
    pub var: VarRef,
    /// Multiplier on the variable.
    pub factor: f32,
    /// Diameter of the sprite at full brightness (m).
    pub size_m: f32,
    /// Shining direction in the object's frame (Bevy axes); zero for omnidirectional lights.
    pub direction: Vec3,
    /// Cosine of the inner cone half-angle (full brightness).
    pub cos_inner: f32,
    /// Cosine of the outer cone half-angle (visibility begins).
    pub cos_outer: f32,
    /// Brightness from the variable, `0..=1` (set after each tick).
    pub level: f32,
}

impl SceneryLight {
    /// Brightness seen from `to_camera` (unit vector from the light to the camera, in the same
    /// frame as `direction`).
    #[must_use]
    pub fn seen_level(&self, to_camera: Vec3) -> f32 {
        if self.direction == Vec3::ZERO {
            return self.level;
        }
        let c = self.direction.dot(to_camera);
        let cone = if c >= self.cos_inner {
            1.0
        } else if c <= self.cos_outer || self.cos_inner <= self.cos_outer {
            0.0
        } else {
            (c - self.cos_outer) / (self.cos_inner - self.cos_outer)
        };
        self.level * cone
    }
}

/// A part whose material has a light map scaled by a variable.
#[derive(Component, Debug, Clone)]
pub struct SceneryLightMap {
    /// Brightness variable.
    pub var: VarRef,
    /// The material may be changed in place: it is shared by every object of this kind and they
    /// all switch together (night lighting), or it is already this part's own copy. Otherwise
    /// the part gets its own copy the first time it changes.
    pub in_place: bool,
    /// Last applied brightness (`NaN` before the first update).
    pub applied: f32,
}

/// Reads a variable for an object.
#[must_use]
pub fn read_var(r: VarRef, engine: Option<&ScriptEngine>, host: &HostValues, phase: f32) -> f32 {
    match r {
        VarRef::Engine(id) => engine.map_or(0.0, |e| e.get_id(id)),
        VarRef::Host(HostVar::Night) => host.night,
        VarRef::Host(HostVar::Phase) => phase,
        VarRef::Host(HostVar::Time) => host.time_s,
        VarRef::Host(HostVar::Zero) => 0.0,
        VarRef::One => 1.0,
    }
}

/// The entities an object update writes to.
#[derive(SystemParam)]
pub struct EffectTargets<'w, 's> {
    rules: Query<
        'w,
        's,
        (
            &'static mut SceneryMeshRule,
            &'static mut Transform,
            &'static mut Visibility,
        ),
    >,
    lights: Query<'w, 's, &'static mut SceneryLight>,
    light_maps: Query<
        'w,
        's,
        (
            &'static mut SceneryLightMap,
            &'static mut MeshMaterial3d<StandardMaterial>,
        ),
    >,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

impl EffectTargets<'_, '_> {
    /// Applies the object's variables to its meshes, lights and light maps. `dt_s` is the time
    /// since the last update (for `maxspeed`).
    pub fn apply(
        &mut self,
        rt: &SceneryObjectRuntime,
        engine: Option<&ScriptEngine>,
        host: &HostValues,
        dt_s: f32,
    ) {
        let phase = rt.phase(host).unwrap_or(0.0);
        let value = |r: VarRef| read_var(r, engine, host, phase);
        for &e in &rt.meshes {
            let Ok((mut rule, mut transform, mut visibility)) = self.rules.get_mut(e) else {
                continue;
            };
            let steps = rule.anim.steps.len();
            let values: Vec<f32> = rule.vars.iter().map(|r| value(*r)).collect();
            if steps > 0 {
                rule.anim.follow(&values[..steps], dt_s);
                let m = animation_matrix_from_values(&rule.anim.steps, &rule.anim.shown);
                let t = Transform::from_matrix(m);
                if *transform != t {
                    *transform = t;
                }
            }
            if !rule.anim.visible.is_empty() {
                let want = if rule.visible_for(&values[steps..]) {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *visibility != want {
                    *visibility = want;
                }
            }
        }
        for &e in &rt.lights {
            if let Ok(mut light) = self.lights.get_mut(e) {
                let level = (value(light.var) * light.factor).clamp(0.0, 1.0);
                if light.level != level {
                    light.level = level;
                }
            }
        }
        for &e in &rt.light_maps {
            let Ok((mut binding, mut material)) = self.light_maps.get_mut(e) else {
                continue;
            };
            let f = value(binding.var).clamp(0.0, MAX_BRIGHTNESS);
            if f == binding.applied {
                continue;
            }
            binding.applied = f;
            let target = LinearRgba::rgb(f, f, f);
            if binding.in_place {
                // Shared by every object with this mesh: change it once, not per object.
                if self
                    .materials
                    .get(&material.0)
                    .is_some_and(|m| m.emissive == target)
                {
                    continue;
                }
            } else if let Some(copy) = self.materials.get(&material.0).cloned() {
                material.0 = self.materials.add(copy);
                binding.in_place = true;
            }
            if let Some(mut m) = self.materials.get_mut(&material.0) {
                m.emissive = target;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn light(direction: Vec3) -> SceneryLight {
        SceneryLight {
            var: VarRef::One,
            factor: 1.0,
            size_m: 0.3,
            direction,
            cos_inner: 20f32.to_radians().cos(),
            cos_outer: 50f32.to_radians().cos(),
            level: 1.0,
        }
    }

    #[test]
    fn a_directional_light_fades_outside_its_cone() {
        let l = light(Vec3::Z);
        assert_eq!(l.seen_level(Vec3::Z), 1.0);
        assert_eq!(l.seen_level(-Vec3::Z), 0.0);
        let mid = Vec3::new(35f32.to_radians().sin(), 0.0, 35f32.to_radians().cos());
        let s = l.seen_level(mid);
        assert!(s > 0.1 && s < 0.9, "{s}");
        assert_eq!(light(Vec3::ZERO).seen_level(-Vec3::Z), 1.0);
    }

    #[test]
    fn visibility_needs_equality_with_the_flag() {
        let rule = SceneryMeshRule {
            anim: MeshAnimation {
                owner: Entity::PLACEHOLDER,
                steps: Vec::new(),
                visible: vec![("season".into(), 0.0)],
                shown: Vec::new(),
            },
            vars: vec![VarRef::Host(HostVar::Zero)],
        };
        assert!(rule.visible_for(&[0.0]));
        assert!(!rule.visible_for(&[1.0]));
    }
}
