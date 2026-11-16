//! The loop of a vehicle under its scripts: one step, the run, the wait for air and the ready state.

use super::*;

impl Loop {
    /// Runs the vehicle's start-up (battery, engine, drive) without waiting for air.
    pub(in super::super) fn ready_basic(&mut self) {
        self.run(12.0);
    }

    /// One step: feed, start-up triggers, scripts, their torque and brakes into the physics.
    pub(in super::super) fn tick(&mut self) {
        let mut inputs = opts_bevy::vehicle_script::host_inputs(&self.controller, None);
        if let Some(r) = &self.route {
            opts_bevy::route::feed_route(&mut inputs, r);
        }
        feed(&mut self.engine, &self.plan, &inputs);
        self.startup.step(self.t, &mut self.engine);
        self.keys.press_next(&mut self.engine, 1.0 / 60.0);
        if let Some(r) = self.route.clone() {
            opts_bevy::route::keep_target(
                &mut self.engine,
                &r,
                &mut self.pending_target_s,
                self.dt,
            );
        }
        self.engine.run_frame(self.dt, false);
        self.controller.sim.script_drive = opts_bevy::vehicle_script::script_outputs(&self.engine);
        self.controller.sim.step(self.dt);
        self.t += self.dt;
    }

    pub(in super::super) fn run(&mut self, seconds: f32) {
        let trace = std::env::var_os("RIG_TRACE").is_some();
        for i in 0..(seconds / self.dt) as usize {
            self.tick();
            if trace && i % (1.0 / self.dt) as usize == 0 {
                eprintln!(
                    "t={:.1} v={:.2} engine_on={} n={:.0} m_wheel={:.0} brake={:.0} park={} gear={}",
                    self.t,
                    self.speed(),
                    self.engine.get("engine_on"),
                    self.engine.get("engine_n"),
                    self.engine.get("m_wheel"),
                    self.controller
                        .sim
                        .script_drive
                        .map_or(0.0, |d| d.brake_force_n),
                    self.engine.get("bremse_feststell"),
                    self.engine.get("antrieb_getr_aktugang"),
                );
            }
        }
    }

    pub(in super::super) fn speed(&self) -> f32 {
        self.controller.sim.chassis.forward_speed_m_s()
    }

    /// Start-up, then the parking brake off and the brake circuits filled until the scripts'
    /// brakes are released (the compressor needs up to two minutes), at most 180 s: the vehicle
    /// stands ready to drive.
    pub(in super::super) fn ready(&mut self) {
        self.run(12.0);
        self.controller.inputs_mut().set_parking_brake(false);
        if self.engine.get("bremse_feststell") > 0.5 {
            self.engine.trigger("parking_brake_toggle");
        }
        self.run(1.0);
        // A vehicle in service has its air tanks at the compressor governor's cut-in (8 bar in
        // the MAN scripts, the governor works between 8 and 9.5): wait for it, as a driver does.
        let air_limit = self.t + 900.0;
        while self.t < air_limit
            && ["bremse_p_tank01", "bremse_p_tank02"]
                .iter()
                .any(|n| self.engine.writes(n) && self.engine.get(n) < AIR_READY_PA)
        {
            self.run(1.0);
        }
        let t_limit = self.t + 170.0;
        while self.t < t_limit {
            let drag = self
                .controller
                .sim
                .script_drive
                .map_or(0.0, |d| d.brake_force_n);
            if drag < 500.0 {
                break;
            }
            self.run(1.0);
        }
    }
}
