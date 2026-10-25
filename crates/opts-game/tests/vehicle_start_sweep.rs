//! Corpus sweep of the start procedures (M-0141): every player bus of the installation through
//! the game's own set-up. Needs an OMSI installation (`OMSI_PATH`); ignored by default.

mod vehicle_rig;

use vehicle_rig::{load, run, DT};

/// Sweep (M-0141): every player bus of the installation through the game's own set-up
/// ([`opts_bevy::vehicle_setup::SetupRun`]) at the default degree (engine running); prints one
/// tab-separated line per vehicle: path, recipe, `engine_on`, engine speed.
#[test]
#[ignore = "corpus sweep; prints SWEEP lines"]
fn sweep_setup_starts_every_bus() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(std::path::PathBuf::from) else {
        return;
    };
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
    buses.sort();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let lines = std::sync::Mutex::new(Vec::new());
    let setup = opts_bevy::vehicle_setup::VehicleSetup::default();
    std::thread::scope(|scope| {
        for _ in 0..12 {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(path) = buses.get(i) else { break };
                let rel = path
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                let Some(mut e) = load(&rel) else { continue };
                if !opts_bevy::vehicle_setup::has_engine(&e) {
                    continue;
                }
                let mut run_ = opts_bevy::vehicle_setup::SetupRun::default();
                let mut now = 0.0;
                while now < 30.0 {
                    let done = run_.advance(&mut e, &setup, now);
                    e.run_frame(DT, false);
                    now += DT;
                    if done && now > 5.0 && e.get("engine_on") > 0.5 {
                        break;
                    }
                }
                run(&mut e, 2.0);
                lines.lock().unwrap().push(format!(
                    "SWEEP\t{rel}\t{}\t{}\t{:.0}",
                    opts_bevy::vehicle_setup::recipe_for(&e).name,
                    e.get("engine_on"),
                    e.get("engine_n")
                ));
            });
        }
    });
    let mut lines = lines.into_inner().unwrap();
    lines.sort();
    for l in lines {
        println!("{l}");
    }
}
