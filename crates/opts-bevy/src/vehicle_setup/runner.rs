//! Step runner that executes a vehicle's start-up recipe over time.

use super::*;

/// The engine speed from which an engine that fired runs on its own: the starter is held until
/// then (released at the first firing, the TH O550 stalls at about 350 rpm).
const SELF_SUSTAINING_RPM: f32 = 400.0;
/// The longest the starter is held after the engine fired, should it never reach that speed.
const AFTER_FIRING_S: f32 = 1.5;

/// Script interlocks a second starter press waits for, where the vehicle's scripts own them: the
/// ABS self-test that the ignition starts (about 6 s on the Studio Polygon Lite) and a
/// wait-to-start lamp. The first press does not wait, so a vehicle that starts at once is never
/// delayed.
const START_INTERLOCKS: [&str; 2] = ["bremse_abs_selftest", "waittostartactive"];
/// The longest a starter step waits for those interlocks before it is pressed anyway.
const INTERLOCK_WAIT_S: f32 = 10.0;

/// Seconds between one step and the next.
const SETTLE_S: f32 = 1.0;
/// The busbar voltage from which the electrics count as on (the scripts' own limit).
const BUSBAR_MIN: f32 = 0.3;
/// The tank when the vehicle's constants name none (litres): a city bus.
const DEFAULT_TANK_L: f32 = 250.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Idle,
    Held,
}

/// The progress of one vehicle through its recipe.
#[derive(Debug, Clone, Default)]
pub struct SetupRun {
    step: usize,
    phase: Phase,
    since: f32,
    /// `now` of the previous call (the frame time of a drag).
    last: f32,
    /// When the engine first fired during the current starter step.
    fired_at: Option<f32>,
    fuel_done: bool,
    lights_done: bool,
}

impl SetupRun {
    /// Takes the next step when it is due. `now` is the vehicle's seconds since it appeared.
    /// Returns `true` once there is nothing more to do at this degree.
    pub fn advance(&mut self, engine: &mut ScriptEngine, setup: &VehicleSetup, now: f32) -> bool {
        if !self.fuel_done && now > 0.2 {
            self.fuel_done = true;
            fill_tank(engine, setup.fuel);
        }
        let recipe = recipe_for(engine);
        let busbar = engine.get("elec_busbar_main");
        let dt = (now - self.last).max(0.0);
        self.last = now;
        if self.phase == Phase::Held {
            let step = recipe.steps[self.step];
            if step.drag_y != 0.0 {
                let moving = now - self.since <= DRAG_S;
                engine.set_system("mouse_x", 0.0);
                engine.set_system(
                    "mouse_y",
                    if moving {
                        step.drag_y * dt / DRAG_S
                    } else {
                        0.0
                    },
                );
                if moving {
                    engine.trigger(&format!("{}_drag", step.trigger));
                }
            }
            let done = if step.until_engine {
                if engine.get("engine_on") > 0.5 && self.fired_at.is_none() {
                    self.fired_at = Some(now);
                }
                let running = self.fired_at.is_some_and(|t| {
                    engine.get("engine_n") >= SELF_SUSTAINING_RPM || now - t >= AFTER_FIRING_S
                });
                running || now - self.since >= step.hold_s
            } else {
                now - self.since >= step.hold_s
            };
            if done {
                if step.drag_y != 0.0 {
                    engine.set_system("mouse_y", 0.0);
                }
                engine.trigger(&format!("{}_off", step.trigger));
                self.fired_at = None;
                self.phase = Phase::Idle;
                self.step += 1;
                self.since = now;
            }
            return false;
        }
        // A vehicle whose scripts have no engine (a tram) has nothing to start: its start step
        // is skipped, and what waits for a busbar its scripts never raise does not wait.
        if let Some(step) = recipe.steps.get(self.step) {
            if step.only_if_off && engine.get("engine_on") > 0.5 {
                self.step += 1;
                return false;
            }
            if step.until_engine && !engine.writes("engine_on") {
                self.step += 1;
                return false;
            }
        }
        let busbar = if engine.writes("elec_busbar_main") {
            busbar
        } else {
            1.0
        };
        if let Some(step) = recipe.steps.get(self.step) {
            if step.from <= setup.degree + 1e-4 {
                let interlocks_clear = !step.only_if_off
                    || now - self.since >= SETTLE_S + INTERLOCK_WAIT_S
                    || START_INTERLOCKS
                        .iter()
                        .all(|v| !engine.writes(v) || engine.get(v) < 0.5);
                let ready = now - self.since >= SETTLE_S
                    && (!step.needs_busbar || busbar > BUSBAR_MIN)
                    && interlocks_clear;
                if ready && now > 0.5 {
                    engine.trigger(step.trigger);
                    self.phase = Phase::Held;
                    self.since = now;
                }
                return false;
            }
        }
        // Nothing left at this degree: the lights, once the electrics are on.
        if !self.lights_done && setup.degree >= ELECTRICS && busbar > BUSBAR_MIN {
            self.lights_done = true;
            match setup.lights {
                StartLights::Off => {}
                StartLights::Parking => {
                    tap_key(engine, "kw_standlicht_toggle");
                }
                StartLights::Dipped => {
                    tap_key(engine, "kw_scheinwerfer_toggle");
                }
            }
        }
        true
    }
}

/// Whether the vehicle's scripts own an engine (`engine_on`): a tram's do not.
#[must_use]
pub fn has_engine(engine: &ScriptEngine) -> bool {
    engine.writes("engine_on")
}

/// Raises a key's trigger and its `_off` (the lights are not held keys).
fn tap_key(engine: &mut ScriptEngine, trigger: &str) {
    engine.trigger(trigger);
    engine.trigger(&format!("{trigger}_off"));
}

/// Sets the tank to `share` of its capacity (the constant `engine_tank_capacity`, or
/// [`DEFAULT_TANK_L`]). The scripts' own start value is a random one; this is the player's.
fn fill_tank(engine: &mut ScriptEngine, share: f32) {
    let capacity = engine
        .consts_mut()
        .scalar("engine_tank_capacity")
        .filter(|c| *c > 1.0)
        .unwrap_or(DEFAULT_TANK_L);
    if engine.writes("engine_tank_content") {
        engine.set("engine_tank_content", capacity * share.clamp(0.0, 1.0));
    }
}
