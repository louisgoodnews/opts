//! The vehicle script corpus families (skipped without `OMSI_PATH`): no unimplemented callbacks.

use bevy::prelude::*;
use opts_core::formats::vehicles::bus::BusManifest;

use super::corpus_support;

/// Verifies M-0093 acceptance criterion: the vehicle scripts of the SD200, NLC (NL/NG) and
/// O530 families report no unimplemented host callbacks when run with vehicle_callbacks.
#[test]
fn vehicle_scripts_corpus_families_report_no_unimplemented_callbacks() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(std::path::PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping corpus test");
        return;
    };
    let install = opts_bevy::OmsiInstall::new(root.clone());
    let candidates = [
        ("SD200", "Vehicles/MAN_SD200/MAN_SD77.bus"),
        ("NL_NG", "Vehicles/MAN_NL_NG/MAN_EN92_main.bus"),
        ("O530", "Vehicles/MB_O530_Facelift/10M Voith 2D M.bus"),
    ];

    for (family, rel) in candidates {
        let Some(bus_path) = install.find_file(rel) else {
            eprintln!("{family} not installed; skipping");
            continue;
        };
        let bus_dir = bus_path.parent().unwrap().to_path_buf();
        let Ok(bytes) = std::fs::read(&bus_path) else {
            continue;
        };
        let Ok(manifest) = BusManifest::parse_bytes(&bytes) else {
            continue;
        };
        let (mut engine, _) =
            opts_sim::script::load_vehicle_scripts(&manifest, |r| install.find_in(&bus_dir, r));

        let hof = corpus_support::find_hof(&bus_dir);
        let script_textures = corpus_support::script_textures(&install, &manifest, &bus_dir);

        let fonts = std::sync::Arc::new(std::sync::RwLock::new(
            opts_bevy::text_texture::load_fonts(&install),
        ));

        let ctx = opts_bevy::host_callbacks::VehicleHostContext {
            seed: 42,
            route: {
                let route = opts_bevy::route::RouteHandle::new(hof);
                route.with_first_route();
                route
            },
            script_textures: std::sync::Arc::new(std::sync::RwLock::new(script_textures)),
            fonts,
            texture_base_dir: Some(bus_dir.join("Texture")),
            ..Default::default()
        };
        engine.set_external_handler(opts_bevy::host_callbacks::vehicle_callbacks_with_context(
            ctx,
        ));
        engine.run_init();
        for _ in 0..60 {
            engine.run_frame(1.0 / 60.0, false);
        }

        let diags = engine.drain_diagnostics();
        let unimplemented: Vec<_> = diags
            .iter()
            .filter(|d| {
                d.message.contains("not implemented") || d.message.contains("has no handler")
            })
            .collect();
        assert!(
            unimplemented.is_empty(),
            "{family} has {} unimplemented callbacks: {:?}",
            unimplemented.len(),
            unimplemented
        );
    }
}
