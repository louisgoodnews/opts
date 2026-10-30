//! The route and physics modes of the script rig (one vehicle each).

use opts_bevy::host_callbacks::vehicle_callbacks;

use super::*;

/// Prints the route report of one vehicle (`--route`, `--timetable`).
pub(super) fn route_mode(args: &Args, bus: &BusManifest, dir: &Path, rel: &str) {
    let code = args.route_code.unwrap_or(0);
    let seed0 = seed_of(rel);
    let depot = depot_for(dir, &args.root, args.hof_filter.as_deref());
    let report = route_report(
        bus,
        dir,
        &args.root,
        depot,
        code,
        seed0,
        args.timetable.as_ref().map(|t| {
            (
                t,
                args.root
                    .join("maps")
                    .join(args.map_name.as_deref().unwrap_or("")),
            )
        }),
    );
    println!(
        "{rel}\n{}",
        serde_json::to_string_pretty(&report).unwrap_or_default()
    );
}

/// Prints the physics report of one vehicle and writes or checks the golden (`--physics`).
pub(super) fn physics_mode(args: &Args, f: &Path, bus: &BusManifest, dir: &Path, rel: &str) {
    let seed0 = seed_of(rel);
    let loader = || {
        let (mut e, _) =
            load_vehicle_scripts(bus, |r| find_ci(dir, r).or_else(|| find_ci(&args.root, r)));
        e.seed_random(seed0 as u32);
        e.set_external_handler(vehicle_callbacks(seed0));
        e
    };
    let trail = f
        .to_string_lossy()
        .strip_suffix("_main.bus")
        .map(|stem| PathBuf::from(format!("{stem}_trail.bus")))
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| BusManifest::parse_bytes(&b).ok());
    let report = physics_report(bus, trail.as_ref(), &loader, args.hz);
    println!(
        "{rel}\n{}",
        serde_json::to_string_pretty(&report).unwrap_or_default()
    );
    let value = serde_json::to_value(&report).unwrap_or_default();
    if let Some(path) = &args.golden {
        let text = serde_json::to_string_pretty(&value).unwrap_or_default();
        std::fs::write(path, format!("{text}\n")).expect("write the golden");
        println!("golden written to {}", path.display());
    }
    if let Some(path) = &args.check {
        let text = std::fs::read_to_string(path).expect("read the golden");
        let want: serde_json::Value = serde_json::from_str(&text).expect("parse the golden");
        let mut diffs = Vec::new();
        physics_diff("", &want, &value, &mut diffs);
        for d in &diffs {
            eprintln!("DIFFERS {d}");
        }
        if !diffs.is_empty() {
            std::process::exit(1);
        }
        println!("physics golden holds ({})", path.display());
    }
}
