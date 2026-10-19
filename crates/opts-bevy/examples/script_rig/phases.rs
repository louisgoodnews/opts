//! The startup and loop phases of the rig.

use super::*;

/// The driver's start-up as the game performs it (battery switch, engine start key, *Drive*).
#[derive(Default)]
pub(super) struct Startup {
    pub(super) stage: u8,
    pub(super) mark: f32,
}

impl Startup {
    pub(super) fn step(&mut self, t: f32, engine: &mut opts_sim::script::ScriptEngine) {
        match self.stage {
            0 if t > 0.5 => {
                engine.trigger("cp_batterietrennschalter_toggle");
                (self.stage, self.mark) = (1, t);
            }
            1 if t - self.mark > 0.3 => {
                engine.trigger("cp_batterietrennschalter_toggle_off");
                (self.stage, self.mark) = (2, t);
            }
            2 if engine.get("elec_busbar_main") > 0.3 && t - self.mark > 1.0 => {
                engine.trigger("kw_m_enginestart");
                (self.stage, self.mark) = (3, t);
            }
            3 if engine.get("engine_on") > 0.5 || t - self.mark > 8.0 => {
                engine.trigger("kw_m_enginestart_off");
                // The automatic leaves neutral only with the brake pressed.
                let brake = engine.get("brake");
                engine.set("brake", 1.0);
                engine.trigger("automatic_d");
                engine.set("brake", brake);
                (self.stage, self.mark) = (4, t);
            }
            // The driver releases the parking brake the scripts start with applied.
            4 if t - self.mark > 0.5 => {
                if engine.get("bremse_feststell") > 0.5 {
                    engine.trigger("parking_brake_toggle");
                }
                (self.stage, self.mark) = (5, t);
            }
            _ => {}
        }
    }
}

/// Tank pressure at which the rig starts measuring (Pa).
const AIR_READY_PA: f32 = 800_000.0;

/// A vehicle's scripts and physics, closed in a loop (`--physics`).
pub(super) struct Loop {
    pub(super) engine: opts_sim::script::ScriptEngine,
    pub(super) plan: FeedPlan,
    pub(super) controller: opts_bevy::VehicleController,
    pub(super) startup: Startup,
    pub(super) t: f32,
    pub(super) dt: f32,
    pub(super) keys: opts_bevy::route::KeyQueue,
    pub(super) route: Option<opts_bevy::route::RouteHandle>,
    pub(super) pending_target_s: f32,
}

impl Loop {
    pub(super) fn new(
        bus: &BusManifest,
        trail: Option<&BusManifest>,
        engine: opts_sim::script::ScriptEngine,
        dt: f32,
    ) -> Self {
        let mut engine = engine;
        let plan = FeedPlan::new(&mut engine);
        let mut sim = opts_sim::VehicleSimulation::from_manifest(bus);
        if let Some(t) = trail {
            // The second unit of an articulated bus: mass and axles (the hitch geometry is a
            // plausible stand-in, the rig drives straight).
            let load = opts_sim::vehicle::consist::UnitLoad::from_manifest(t, false);
            sim.attach_trailer(
                bevy::math::DVec2::new(0.0, -3.5),
                bevy::math::DVec2::new(0.0, 1.5),
                6.0,
                52.5,
                Some(load),
            );
        }
        let mut controller = opts_bevy::VehicleController::new(sim);
        controller
            .inputs_mut()
            .set_gear(opts_sim::GearPosition::Drive);
        controller.inputs_mut().set_parking_brake(true);
        let first = opts_bevy::vehicle_script::host_inputs(&controller, None);
        feed(&mut engine, &plan, &first);
        engine.run_init();
        Self {
            engine,
            plan,
            controller,
            startup: Startup::default(),
            t: 0.0,
            dt,
            keys: opts_bevy::route::KeyQueue::default(),
            route: None,
            pending_target_s: 0.0,
        }
    }

    /// Runs the vehicle's start-up (battery, engine, drive) without waiting for air.
    pub(super) fn ready_basic(&mut self) {
        self.run(12.0);
    }

    /// One step: feed, start-up triggers, scripts, their torque and brakes into the physics.
    pub(super) fn tick(&mut self) {
        let mut inputs = opts_bevy::vehicle_script::host_inputs(&self.controller, None);
        if let Some(r) = &self.route {
            opts_bevy::route::feed_route(&mut inputs, r);
        }
        feed(&mut self.engine, &self.plan, &inputs);
        self.startup.step(self.t, &mut self.engine);
        self.keys.press_next(&mut self.engine);
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

    pub(super) fn run(&mut self, seconds: f32) {
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

    pub(super) fn speed(&self) -> f32 {
        self.controller.sim.chassis.forward_speed_m_s()
    }

    /// Start-up, then the parking brake off and the brake circuits filled until the scripts'
    /// brakes are released (the compressor needs up to two minutes), at most 180 s: the vehicle
    /// stands ready to drive.
    pub(super) fn ready(&mut self) {
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
