//! The startup and loop phases of the rig.

use super::*;

mod drive;

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
}
