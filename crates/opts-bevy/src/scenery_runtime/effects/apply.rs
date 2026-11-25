//! Applying an object's variables to its meshes, lights and light maps, after each tick.

use super::*;

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
                let targets = &values[..steps];
                rule.anim.follow(targets, dt_s);
                if rule.last_target.as_slice() != targets {
                    tracing::debug!(owner = ?rule.anim.owner, targets = ?targets, shown = ?rule.anim.shown, "scenery animation target");
                    rule.last_target = targets.to_vec();
                }
                tracing::trace!(owner = ?rule.anim.owner, shown = ?rule.anim.shown, "scenery animation value");
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
