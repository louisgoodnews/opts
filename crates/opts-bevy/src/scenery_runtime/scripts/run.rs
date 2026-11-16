//! One tick of a scenery object's script: the host values, the overrides, `{init}` and `{frame}`.

use super::*;

impl SceneryScripts {
    /// Feeds the host values, runs `{init}` on the first call and `{frame}` with step `dt_s`.
    /// `phase` is the object's `TrafficLightPhase`. Returns the number of new run-time
    /// diagnostics.
    pub fn tick(
        &mut self,
        host: &HostValues,
        phase: Option<f32>,
        overrides: &SceneryVariableOverrides,
        dt_s: f32,
    ) -> usize {
        let ids = self.ids;
        let first = !self.initialised;
        self.initialised = true;
        self.ticks += 1;
        let Ok(e) = self.engine.get_mut() else {
            return 0;
        };
        e.set_system_id(ids.time, host.time_s);
        e.set_system_id(ids.day, host.day);
        e.set_system_id(ids.month, host.month);
        e.set_system_id(ids.year, host.year);
        e.set_system_id(ids.day_of_year, host.day_of_year);
        e.set_system_id(ids.sun_alt, host.sun_alt_deg);
        e.set_system_id(ids.temperature, host.temperature_c);
        e.set_system_id(ids.abs_hum, host.abs_hum);
        e.set_system_id(ids.precip_type, host.precip_type);
        e.set_system_id(ids.precip_rate, host.precip_rate);
        e.set_id(ids.night, host.night);
        if let Some(p) = phase {
            e.set_id(ids.phase, p);
        }
        for (name, value) in &overrides.0 {
            e.set(name, *value);
        }
        if first {
            e.run_init();
        }
        e.run_frame(dt_s, false);
        // The events the script raised (`(T.L.name)`, lower case) wait for the sound layer
        // (`opts_game::scenery_sound`), which plays the `[trigger]`s of the object's sounds.
        for ev in e.drain_events() {
            if self.raised.len() < MAX_RAISED {
                self.raised.push(ev.name);
            }
        }
        e.drain_messages();
        let new = e.drain_diagnostics();
        let count = new.len();
        for d in new {
            tracing::debug!(file = %d.file, line = d.line, "scenery script: {}", d.message);
            if self.diagnostics.len() < MAX_KEPT_DIAGNOSTICS {
                self.diagnostics.push(d);
            }
        }
        count
    }
}
