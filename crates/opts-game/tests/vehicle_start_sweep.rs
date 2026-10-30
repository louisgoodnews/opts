//! Corpus sweep of the start procedures (M-0141): every player bus of the installation through
//! the game's own set-up. Needs an OMSI installation (`OMSI_PATH`); ignored by default.

mod vehicle_rig;

use vehicle_rig::{load, run, DT};

/// Collects every `.bus` file below `dir`.
fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("bus"))
        {
            out.push(path);
        }
    }
}

/// Runs one bus through the set-up and formats its `SWEEP` line (`None`: not loadable, no engine).
fn sweep_line(
    root: &std::path::Path,
    path: &std::path::Path,
    setup: &opts_bevy::vehicle_setup::VehicleSetup,
) -> Option<String> {
    let rel = path
        .strip_prefix(root)
        .unwrap()
        .to_string_lossy()
        .to_string();
    let mut e = load(&rel)?;
    if !opts_bevy::vehicle_setup::has_engine(&e) {
        return None;
    }
    let mut run_ = opts_bevy::vehicle_setup::SetupRun::default();
    let mut now = 0.0;
    let mut starter_seen = 0.0_f32;
    while now < 40.0 {
        let done = run_.advance(&mut e, setup, now);
        e.run_frame(DT, false);
        starter_seen = starter_seen.max(e.get("engine_starter"));
        now += DT;
        if done && now > 5.0 && opts_bevy::vehicle_setup::ready(&e) {
            break;
        }
    }
    run(&mut e, 2.0);
    Some(format!(
        "SWEEP\t{rel}\t{}\t{}\t{:.0}\t{:.2}\t{}\t{}\t{:.0}",
        opts_bevy::vehicle_setup::recipe_for(&e).name,
        u8::from(opts_bevy::vehicle_setup::ready(&e)),
        e.get("engine_n"),
        e.get("elec_busbar_main"),
        starter_seen,
        e.get("antrieb_getr_gangwahl"),
        e.get("engine_tank_content")
    ))
}

/// Sweep (M-0141): every player bus of the installation through the game's own set-up
/// ([`opts_bevy::vehicle_setup::SetupRun`]) at the default degree (engine running); prints one
/// tab-separated line per vehicle: path, recipe, ready to drive (`vehicle_setup::ready`: the
/// engine runs, or a serial hybrid is ready), engine speed, then where a start
/// got stuck: busbar voltage, whether the starter ever engaged, gear selector, tank content.
/// `SWEEP_FILTER` (a substring of the path) limits the run.
#[test]
#[ignore = "corpus sweep; prints SWEEP lines"]
fn sweep_setup_starts_every_bus() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(std::path::PathBuf::from) else {
        return;
    };
    let mut buses = Vec::new();
    walk(&root.join("Vehicles"), &mut buses);
    buses.retain(|p| {
        let name = p
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_ascii_lowercase();
        !["_ki", "ki_", "_ai", "ai_", "parked", "_trail"]
            .iter()
            .any(|tag| name.contains(tag))
    });
    if let Ok(filter) = std::env::var("SWEEP_FILTER") {
        buses.retain(|p| p.to_string_lossy().contains(&filter));
    }
    buses.sort();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let lines = std::sync::Mutex::new(Vec::new());
    let setup = opts_bevy::vehicle_setup::VehicleSetup::default();
    std::thread::scope(|scope| {
        for _ in 0..12 {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(path) = buses.get(i) else { break };
                if let Some(line) = sweep_line(&root, path, &setup) {
                    lines.lock().unwrap().push(line);
                }
            });
        }
    });
    let mut lines = lines.into_inner().unwrap();
    lines.sort();
    for l in lines {
        println!("{l}");
    }
}
