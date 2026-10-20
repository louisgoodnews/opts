//! Part census of one vehicle (M-0138): every `[mesh]` of its model config with the file it
//! resolves to, whether it parses, its decode layout, vertex and triangle counts, bounding box
//! and, per material slot, the texture name, whether the texture resolves (with the engine's own
//! search order), the colours and the alpha mode the config asks for, plus the visibility rules,
//! animations and viewpoint flags. Prints one TSV row per mesh and a summary on stderr; no GPU.
//! Usage: `cargo run --release -p opts-bevy --example vehicle_census -- <OMSI root> <.bus file>`.

use std::path::{Path, PathBuf};

use opts_bevy::{find_texture, mesh::parse_mesh_file, OmsiInstall};
use opts_core::formats::o3d::VertexLayout;
use opts_core::formats::vehicles::{bus::BusManifest, model::ModelConfig};

fn main() {
    let mut args = std::env::args().skip(1);
    let root = args.next().expect("OMSI root");
    let bus_path = PathBuf::from(args.next().expect(".bus file"));
    let install = OmsiInstall::new(&root);
    let bus = BusManifest::parse_bytes(&std::fs::read(&bus_path).expect("read .bus"))
        .expect("parse .bus");
    let bus_dir = bus_path.parent().unwrap_or(Path::new("."));
    let cfg_rel = bus.model_cfg.clone().expect("[model]");
    let cfg_path = install.find_in(bus_dir, &cfg_rel).expect("model config");
    let model = ModelConfig::parse_bytes(&std::fs::read(&cfg_path).expect("read model config"))
        .expect("parse model config");
    let model_dir = cfg_path.parent().unwrap_or(Path::new("."));
    // The folders the game searches for a vehicle's textures (`omsi_world/vehicle.rs`).
    let tex_dirs = vec![
        install
            .find_in(bus_dir, "texture")
            .unwrap_or_else(|| bus_dir.join("texture")),
        model_dir.join("texture"),
    ];

    println!("lod\tmesh\tstatus\tlayout\tverts\ttris\tviewpoint\tshadow\trules\tanims\tslot\ttexture\ttexture_file\tcfg_alpha\tdiffuse_alpha\temissive\talpha_var");
    let (mut meshes, mut missing_files, mut bad_parse, mut missing_tex) = (0, 0, 0, 0);
    for (li, lod) in model.lods.iter().enumerate() {
        for m in &lod.meshes {
            meshes += 1;
            let rules = m.visibility.len();
            let anims = m.anims.len();
            let vp = m.viewpoint.raw();
            let Some(path) = install.find_in(model_dir, &m.path) else {
                missing_files += 1;
                println!(
                    "{li}\t{}\tFILE_MISSING\t\t\t\t{vp}\t{}\t{rules}\t{anims}\t\t\t\t\t\t",
                    m.path, m.isshadow
                );
                continue;
            };
            let bytes = std::fs::read(&path).unwrap_or_default();
            let parsed = match parse_mesh_file(&path, &bytes) {
                Ok(p) => p,
                Err(e) => {
                    bad_parse += 1;
                    println!(
                        "{li}\t{}\tPARSE_ERROR {e}\t\t\t\t{vp}\t{}\t{rules}\t{anims}\t\t\t\t\t\t",
                        m.path, m.isshadow
                    );
                    continue;
                }
            };
            for o in &parsed {
                let layout = match o.vertex_layout() {
                    VertexLayout::Unreadable => "UNREADABLE",
                    _ if o.exact_decoded => "exact",
                    _ => "plain",
                };
                for (slot, mat) in o.materials.iter().enumerate() {
                    let tex = mat.primary_texture().unwrap_or("");
                    let found = find_texture(&install, tex, &tex_dirs);
                    let named = !tex.trim().is_empty() && !tex.chars().all(|c| c == '-');
                    if named
                        && found.is_none()
                        && !tex.starts_with('\\')
                        && !tex.to_lowercase().starts_with("textfeld")
                    {
                        missing_tex += 1;
                    }
                    let cfg = m.materials.iter().find(|c| c.slot_index as usize == slot);
                    let cfg_alpha = cfg
                        .and_then(|c| c.alpha_mode)
                        .map_or(String::new(), |a| a.to_string());
                    let alpha_var = cfg
                        .and_then(|c| c.alpha_scale_var.clone())
                        .unwrap_or_default();
                    println!(
                        "{li}\t{}\tok\t{layout}\t{}\t{}\t{vp}\t{}\t{rules}\t{anims}\t{slot}\t{tex}\t{}\t{cfg_alpha}\t{:.2}\t{:.2}/{:.2}/{:.2}\t{alpha_var}",
                        m.path,
                        o.vertices.len(),
                        o.indices.len(),
                        m.isshadow,
                        found.as_ref().map_or("NOT_FOUND".to_owned(), |p| p
                            .strip_prefix(&root)
                            .unwrap_or(p)
                            .display()
                            .to_string()),
                        mat.diffuse[3],
                        mat.emissive[0], mat.emissive[1], mat.emissive[2],
                    );
                }
            }
        }
    }
    eprintln!("meshes {meshes}, files missing {missing_files}, parse errors {bad_parse}, named textures not found {missing_tex}");
}
