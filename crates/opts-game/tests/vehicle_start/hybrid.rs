//! The MB O530 G BlueTEC Hybrid (`18M GDH Main.bus`, M-0141): a serial hybrid whose diesel stays
//! off at standstill. Its start procedure is the battery switch and one press of the start key,
//! which toggles drive readiness (`startup_mode`); the game's set-up runs that recipe at the ready
//! degree on the vehicle's own scripts.
use super::vehicle_rig::DT;
use super::*;
use opts_bevy::vehicle_setup::{ready, recipe_for, SetupRun, VehicleSetup, READY};

const GDH: &str = "Vehicles/MB_O530_Facelift/18M GDH Main.bus";

#[test]
fn mb_o530_gdh_reaches_ready_to_drive_through_the_set_up() {
    let Some(mut e) = load(GDH) else {
        eprintln!("OMSI_PATH / MB O530 GDH not available; skipping");
        return;
    };
    run(&mut e, 1.0);
    assert_eq!(e.get("startup_mode"), 0.0, "cold: not ready");
    assert!(!ready(&e), "cold: not ready to drive");

    let setup = VehicleSetup {
        degree: READY,
        ..VehicleSetup::default()
    };
    let mut set_up = SetupRun::default();
    let mut now = 0.0;
    while now < 60.0 {
        let done = set_up.advance(&mut e, &setup, now);
        e.run_frame(DT, false);
        now += DT;
        if done && now > 5.0 {
            break;
        }
    }
    run(&mut e, 2.0);

    assert_eq!(recipe_for(&e).name, "serial hybrid start button");
    assert!(e.get("elec_busbar_main") > 0.3, "electrics on");
    assert!(e.get("startup_mode") > 0.5, "drive readiness set");
    assert!(ready(&e), "ready to drive");
    assert_eq!(
        e.get("engine_on"),
        0.0,
        "the diesel stays off at standstill"
    );
}
