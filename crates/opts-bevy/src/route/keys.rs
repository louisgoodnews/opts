//! The keys a driver presses to select a route, one per frame: on the MAN-style IBIS
//! (`ibis_setmode_linie_kurs`, `ibis_eingabe`, `ibis_<digit>`) and on the ticket printer that
//! the MAN New Lion's City uses as its IBIS (`Drucker_*`: log in with the driver number and PIN
//! from the vehicle's constants, key the line, `Drucker_Tab`, key the course, confirm).

use std::collections::VecDeque;

use opts_sim::script::ScriptEngine;

use super::{RouteError, RouteSelection};

/// Seconds a step waits for the printer (booting takes about 10) before the rest is dropped.
const WAIT_LIMIT_S: f32 = 40.0;

/// The keys of a selection on the MAN-style IBIS: line and suffix as `line * 100 + suffix`, then
/// the route (course), then the destination (terminus code). Names are the triggers of the
/// scripts.
#[must_use]
pub fn ibis_keys(selection: &RouteSelection) -> Vec<String> {
    let digits = |n: i32| -> Vec<String> {
        n.max(0)
            .to_string()
            .chars()
            .map(|c| format!("ibis_{c}"))
            .collect()
    };
    let mut keys = vec!["ibis_setmode_linie_kurs".to_string()];
    keys.extend(digits(selection.line * 100));
    keys.push("ibis_eingabe".into());
    keys.push("ibis_setmode_route".into());
    keys.extend(digits(selection.course));
    keys.push("ibis_eingabe".into());
    keys.push("ibis_setmode_ziel".into());
    keys.extend(digits(selection.terminus_code));
    keys.push("ibis_eingabe".into());
    keys
}

/// Whether the scripts have the keypad [`ibis_keys`] presses.
#[must_use]
pub fn has_ibis_keypad(engine: &ScriptEngine) -> bool {
    ["ibis_eingabe", "ibis_setmode_linie_kurs", "ibis_0"]
        .iter()
        .all(|t| engine.has_trigger(t))
}

/// Whether the scripts have the ticket printer's keypad.
#[must_use]
pub fn has_drucker_keypad(engine: &ScriptEngine) -> bool {
    [
        "drucker_anmelden_enter",
        "drucker_lk_bestaetigen",
        "drucker_tab",
        "drucker_num0",
    ]
    .iter()
    .all(|t| engine.has_trigger(t))
}

/// One step of a typed selection.
#[derive(Debug, Clone, PartialEq)]
enum Step {
    /// Raise this trigger.
    Key(String),
    /// Wait until the variable reaches the value (the printer boots, the login is accepted).
    Until(&'static str, f32),
}

/// The printer's keys for a selection, logging in first when the printer is not logged in.
fn drucker_steps(engine: &ScriptEngine, selection: &RouteSelection) -> Vec<Step> {
    let digits = |n: i64| -> Vec<Step> {
        n.max(0)
            .to_string()
            .chars()
            .map(|c| Step::Key(format!("drucker_num{c}")))
            .collect()
    };
    let key = |name: &str| Step::Key(name.to_owned());
    let mut steps = vec![Step::Until("drucker_mode", 1.0)];
    if engine.get("drucker_loggedin") < 0.5 {
        let number = engine.const_scalar("drucker_fahrernummer").unwrap_or(0.0);
        let pin = engine.const_scalar("drucker_fahrerpin").unwrap_or(0.0);
        steps.extend(digits(number as i64));
        steps.push(key("drucker_anmelden_enter"));
        steps.extend(digits(pin as i64));
        steps.push(key("drucker_anmelden_enter"));
    }
    steps.push(Step::Until("drucker_mode", 2.0));
    steps.extend(digits(i64::from(selection.line)));
    steps.push(key("drucker_tab"));
    steps.extend(digits(i64::from(selection.course)));
    steps.push(key("drucker_lk_bestaetigen"));
    steps
}

/// Keys still to be pressed, one per frame (the scripts see every key in a frame of its own, as
/// with a person's finger).
#[derive(Debug, Clone, Default)]
pub struct KeyQueue {
    steps: VecDeque<Step>,
    waited_s: f32,
}

impl KeyQueue {
    /// Queues the keys of a selection for this vehicle.
    ///
    /// # Errors
    /// [`RouteError::NoKeypad`] when the scripts have no keypad of a known kind.
    pub fn type_selection(
        &mut self,
        engine: &ScriptEngine,
        selection: &RouteSelection,
    ) -> Result<(), RouteError> {
        self.steps.clear();
        self.waited_s = 0.0;
        if has_ibis_keypad(engine) {
            self.steps
                .extend(ibis_keys(selection).into_iter().map(Step::Key));
        } else if has_drucker_keypad(engine) {
            self.steps.extend(drucker_steps(engine, selection));
        } else {
            return Err(RouteError::NoKeypad);
        }
        Ok(())
    }

    /// Presses the next key, if any (`dt` seconds passed since the last call: a wait for the
    /// printer counts them); returns whether one was pressed.
    pub fn press_next(&mut self, engine: &mut ScriptEngine, dt: f32) -> bool {
        match self.steps.front() {
            Some(Step::Until(var, at_least)) => {
                if engine.get(var) >= *at_least {
                    self.steps.pop_front();
                    self.waited_s = 0.0;
                } else {
                    self.waited_s += dt;
                    if self.waited_s > WAIT_LIMIT_S {
                        self.steps.clear();
                    }
                }
                false
            }
            Some(Step::Key(_)) => {
                let Some(Step::Key(k)) = self.steps.pop_front() else {
                    return false;
                };
                engine.trigger(&k);
                true
            }
            None => false,
        }
    }

    /// Whether keys are still waiting.
    #[must_use]
    pub fn is_typing(&self) -> bool {
        !self.steps.is_empty()
    }
}

#[cfg(test)]
mod tests;
