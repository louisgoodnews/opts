//! Script variables for vehicles without a script engine (AI traffic in kinematic mode).
//!
//! The animation, visibility and light systems read their variables through a
//! [`VariableSource`]: the vehicle's script engine when it has one, otherwise a plain table the
//! host fills ([`AiVariables`]). The names are the ones OMSI's AI vehicle models use
//! (`docs/src/architecture/ai-traffic.md`, §2.2): `wheel_rotation_<axle>_<l|r>`,
//! `axle_steering_<axle>_<l|r>`, `lights_blinker_l/r`, `ai_light`, `ai_pax1…4`, …

use std::collections::HashMap;
use std::sync::MutexGuard;

use bevy::prelude::*;
use opts_sim::script::ScriptEngine;

use crate::vehicle_script::VehicleScripts;

/// A table of named values, looked up case-insensitively; missing names read `0`.
#[derive(Component, Debug, Clone, Default)]
pub struct AiVariables(HashMap<String, f32>);

impl AiVariables {
    /// Sets `name` (stored lower-case).
    pub fn set(&mut self, name: &str, value: f32) {
        match self.0.get_mut(name) {
            Some(v) => *v = value,
            None => {
                self.0.insert(name.to_ascii_lowercase(), value);
            }
        }
    }

    /// Every value set so far, with its (lower case) name.
    pub fn iter(&self) -> impl Iterator<Item = (&str, f32)> {
        self.0.iter().map(|(k, v)| (k.as_str(), *v))
    }

    /// Reads `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> f32 {
        self.0
            .get(name)
            .or_else(|| self.0.get(&name.to_ascii_lowercase()))
            .copied()
            .unwrap_or(0.0)
    }
}

/// Where a vehicle's variables come from.
pub enum VariableSource<'a> {
    /// The vehicle's script engine (locked while the source lives).
    Engine(MutexGuard<'a, ScriptEngine>),
    /// A host-filled table.
    Table(&'a AiVariables),
}

impl<'a> VariableSource<'a> {
    /// The script engine if there is one, else the table; `None` for neither (or a poisoned lock).
    #[must_use]
    pub fn of(scripts: Option<&'a VehicleScripts>, table: Option<&'a AiVariables>) -> Option<Self> {
        match (scripts, table) {
            (Some(s), _) => s.engine.lock().ok().map(Self::Engine),
            (None, Some(t)) => Some(Self::Table(t)),
            (None, None) => None,
        }
    }

    /// Reads a variable.
    #[must_use]
    pub fn get(&self, name: &str) -> f32 {
        match self {
            Self::Engine(e) => e.get(name),
            Self::Table(t) => t.get(name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookups_ignore_case_and_default_to_zero() {
        let mut t = AiVariables::default();
        t.set("Wheel_Rotation_0_L", 1.5);
        assert_eq!(t.get("wheel_rotation_0_l"), 1.5);
        assert_eq!(t.get("WHEEL_ROTATION_0_L"), 1.5);
        t.set("wheel_rotation_0_l", 2.0);
        assert_eq!(t.get("Wheel_Rotation_0_L"), 2.0);
        assert_eq!(t.get("missing"), 0.0);
    }

    #[test]
    fn the_engine_wins_over_the_table() {
        let (engine, _) = ScriptEngine::from_sources(&[("t.osc", "{init}\n 3 (S.L.x)\n{end}")]);
        let scripts = VehicleScripts::new(engine);
        scripts.engine.lock().unwrap().run_init();
        let mut table = AiVariables::default();
        table.set("x", 9.0);
        assert_eq!(
            VariableSource::of(Some(&scripts), Some(&table))
                .unwrap()
                .get("x"),
            3.0
        );
        assert_eq!(
            VariableSource::of(None, Some(&table)).unwrap().get("x"),
            9.0
        );
        assert!(VariableSource::of(None, None).is_none());
    }
}
