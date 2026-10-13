//! Real-data check: programs of one `[traffic_lights_group]` run on the group's cycle time.
//! Skipped unless `OMSI_PATH` points at an installation.

use std::path::PathBuf;

use opts_core::formats::scenery::sco::SceneryObject;
use opts_traffic::signals::{Aspect, SignalProgram};

#[test]
fn heer_wilhelm_cars_and_pedestrians_never_conflict() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let path = root.join("Sceneryobjects/Kreuz_MC/Kreuz_Heer_Wilhelm.sco");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("{} missing; skipping", path.display());
        return;
    };
    let sco = SceneryObject::parse(&bytes).expect("parse");
    // Declaration order: 0 Main, 1 Side, 2 Main_Ped, 3 Side_Ped, 4 VacArrow_Main.
    let prog = |i: usize| {
        let (g, p) = sco.traffic_light(i).expect("program");
        let phases = p
            .phases
            .iter()
            .map(|ph| (ph.state, f64::from(ph.duration_sec)))
            .collect();
        SignalProgram::in_period(phases, f64::from(g.period_sec()), 0.0).expect("program")
    };
    let (main, side, main_ped, side_ped) = (prog(0), prog(1), prog(2), prog(3));
    for p in [&main, &side, &main_ped, &side_ped] {
        assert!((p.cycle() - 83.0).abs() < 0.01, "cycle {}", p.cycle());
    }
    let go = |p: &SignalProgram, t: f64| p.aspect_at(t) == Aspect::Go;
    for k in 0..4 * 83 * 2 {
        let t = f64::from(k) * 0.5 + 0.25;
        assert!(!(go(&main, t) && go(&side, t)), "main/side green together at {t}");
        assert!(!(go(&side, t) && go(&main_ped, t)), "side cars vs main ped at {t}");
        assert!(!(go(&main, t) && go(&side_ped, t)), "main cars vs side ped at {t}");
    }
}
