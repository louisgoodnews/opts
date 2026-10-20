//! BRT C2 census gate (M-0138): every `[mesh]` of the solo C2's model config resolves to a file
//! that parses as a plain mesh, and every texture the meshes name is found by the engine's own
//! search. Skipped without `OMSI_PATH` or without the add-on.

use std::path::{Path, PathBuf};

use opts_bevy::{find_texture, mesh::parse_mesh_file, OmsiInstall};
use opts_core::formats::o3d::VertexLayout;
use opts_core::formats::vehicles::{bus::BusManifest, model::ModelConfig};

/// Texture names that are not files: script slots and run-time text fields.
fn is_virtual(name: &str) -> bool {
    let n = name.trim();
    n.is_empty()
        || n.chars().all(|c| c == '-')
        || n.starts_with('\\')
        || n.to_lowercase().starts_with("textfeld")
}

#[test]
fn every_part_of_the_solo_c2_resolves_parses_and_has_its_textures() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let install = OmsiInstall::new(&root);
    let Some(bus_path) = install.find_file("Vehicles/BRT_C2-Solo/C2_2T.bus") else {
        eprintln!("BRT_C2-Solo is not installed; skipping");
        return;
    };
    let bus =
        BusManifest::parse_bytes(&std::fs::read(&bus_path).expect("read .bus")).expect(".bus");
    let bus_dir = bus_path.parent().unwrap_or(Path::new("."));
    let cfg = install
        .find_in(bus_dir, &bus.model_cfg.expect("[model]"))
        .expect("model config");
    let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).expect("read cfg")).expect("cfg");
    let model_dir = cfg.parent().unwrap_or(Path::new("."));
    // The folders the game searches for a vehicle's textures (`omsi_world/vehicle.rs`).
    let tex_dirs = vec![
        install
            .find_in(bus_dir, "texture")
            .unwrap_or_else(|| bus_dir.join("texture")),
        model_dir.join("texture"),
    ];

    let (mut meshes, mut problems) = (0, Vec::new());
    for lod in &model.lods {
        for m in &lod.meshes {
            meshes += 1;
            let Some(path) = install.find_in(model_dir, &m.path) else {
                problems.push(format!("{}: file missing", m.path));
                continue;
            };
            let bytes = std::fs::read(&path).unwrap_or_default();
            let parsed = match parse_mesh_file(&path, &bytes) {
                Ok(p) => p,
                Err(e) => {
                    problems.push(format!("{}: {e}", m.path));
                    continue;
                }
            };
            for o in &parsed {
                if o.vertex_layout() == VertexLayout::Unreadable {
                    problems.push(format!("{}: unreadable vertex layout", m.path));
                }
                for mat in &o.materials {
                    let Some(tex) = mat.primary_texture().filter(|t| !is_virtual(t)) else {
                        continue;
                    };
                    if find_texture(&install, tex, &tex_dirs).is_none() {
                        problems.push(format!("{}: texture {tex} not found", m.path));
                    }
                }
            }
        }
    }
    assert!(
        meshes >= 500,
        "the C2 declares about 545 meshes, found {meshes}"
    );
    assert!(
        problems.is_empty(),
        "{} unexplained part(s):\n{}",
        problems.len(),
        problems.join("\n")
    );
}
