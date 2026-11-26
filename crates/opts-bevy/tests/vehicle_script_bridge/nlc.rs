//! The NLC dash and matrix script textures receive drawing (skipped without `OMSI_PATH`).

use super::BusManifest;
use bevy::prelude::*;

#[test]
fn nlc_dash_and_matrix_script_textures_receive_drawing() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(std::path::PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping NLC test");
        return;
    };
    let install = opts_bevy::OmsiInstall::new(root);
    let Some(bus_path) = install.find_file("Vehicles/MAN_NL_NG/MAN_EN92_main.bus") else {
        eprintln!("MAN_EN92_main.bus not installed; skipping");
        return;
    };
    let bus_dir = bus_path.parent().unwrap().to_path_buf();
    let Ok(bytes) = std::fs::read(&bus_path) else {
        return;
    };
    let Ok(manifest) = BusManifest::parse_bytes(&bytes) else {
        return;
    };
    let (mut engine, _) = opts_sim::script::load_vehicle_scripts(&manifest, |r| {
        super::read_script(install.find_in(&bus_dir, r))
    });

    // Script textures: 128x32 and 1024x256
    let tex0 = opts_core::formats::script_texture::CpuScriptTexture::new(128, 32);
    let tex1 = opts_core::formats::script_texture::CpuScriptTexture::new(1024, 256);
    let canvases = std::sync::Arc::new(std::sync::RwLock::new(vec![tex0, tex1]));
    let fonts = std::sync::Arc::new(std::sync::RwLock::new(opts_bevy::text_texture::load_fonts(
        &install,
    )));

    // Find Aachen.hof
    let mut hof = None;
    if let Some(hof_path) = install.find_file("Vehicles/MAN_NL_NG/Aachen.hof") {
        if let Ok(hbytes) = std::fs::read(&hof_path) {
            if let Ok(h) = opts_core::formats::hof::HofFile::parse_bytes(&hbytes) {
                hof = Some(std::sync::Arc::new(h));
            }
        }
    }

    let ctx = opts_bevy::host_callbacks::VehicleHostContext {
        seed: 101,
        route: {
            let route = opts_bevy::route::RouteHandle::new(hof);
            route.with_first_route();
            route
        },
        script_textures: canvases.clone(),
        fonts,
        texture_base_dir: Some(bus_dir.join("Texture")),
        ..Default::default()
    };

    engine.set_external_handler(opts_bevy::host_callbacks::vehicle_callbacks_with_context(
        ctx,
    ));
    engine.set_string("SetLineTo", "33");
    engine.set_string("Ident", "3301");
    engine.set("schedule_active", 1.0);
    engine.run_init();

    for _ in 0..120 {
        engine.run_frame(1.0 / 60.0, false);
    }

    // Verify 0 unimplemented callbacks
    let diags = engine.drain_diagnostics();
    let unimplemented: Vec<_> = diags
        .iter()
        .filter(|d| d.message.contains("not implemented") || d.message.contains("has no handler"))
        .collect();
    assert!(
        unimplemented.is_empty(),
        "NLC has {} unimplemented callbacks: {:?}",
        unimplemented.len(),
        unimplemented
    );
}
