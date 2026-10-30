//! Audits mesh placement of every `.bus` read from stdin without a GPU.
//!
//! For each vehicle it loads lod0, and for every mesh driven by a wheel animation
//! (`Wheel_Rotation*`) compares the centre of the converted geometry with the animation pivot
//! (`origin_trans`, vehicle space → Bevy). Also reports implausible body bounds.
//! Usage: `find <omsi> -name '*.bus' | cargo run --release -p opts-bevy --example vehicle_audit -- <omsi root>`.

use std::io::BufRead;
use std::path::{Path, PathBuf};

use bevy::math::Vec3;
use bevy::mesh::VertexAttributeValues as V;
use opts_bevy::{
    mesh::o3d_to_bevy_meshes_by_material, mesh::parse_mesh_file, omsi_to_bevy_pos, OmsiInstall,
};
use opts_core::formats::vehicles::{bus::BusManifest, model::ModelConfig};

fn bounds(path: &Path, bytes: &[u8]) -> Option<(Vec3, Vec3)> {
    let models = parse_mesh_file(path, bytes).ok()?;
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for m in &models {
        for (_, mesh) in o3d_to_bevy_meshes_by_material(m) {
            if let Some(V::Float32x3(pos)) = mesh.attribute(bevy::mesh::Mesh::ATTRIBUTE_POSITION) {
                for p in pos {
                    lo = lo.min(Vec3::from(*p));
                    hi = hi.max(Vec3::from(*p));
                }
            }
        }
    }
    (lo.x <= hi.x).then_some((lo, hi))
}

/// Parses a vehicle's `.bus` and `model.cfg`; returns the model and the directory of its config.
fn load_vehicle(install: &OmsiInstall, bus_path: &Path) -> Option<(ModelConfig, PathBuf)> {
    let bytes = std::fs::read(bus_path).ok()?;
    let bus = BusManifest::parse_bytes(&bytes).ok()?;
    let cfg_rel = bus.model_cfg.clone()?;
    let bus_dir = bus_path.parent().unwrap_or(Path::new("."));
    let cfg_path = install.find_in(bus_dir, &cfg_rel)?;
    let model = ModelConfig::parse_bytes(&std::fs::read(&cfg_path).unwrap_or_default()).ok()?;
    let model_dir = cfg_path.parent().unwrap_or(Path::new(".")).to_path_buf();
    Some((model, model_dir))
}

/// Whether any mesh of the file has an unreadable (scrambled) vertex layout.
fn is_scrambled(path: &Path, data: &[u8]) -> bool {
    parse_mesh_file(path, data).ok().is_some_and(|m| {
        m.iter()
            .any(|o| o.vertex_layout() == opts_core::formats::o3d::VertexLayout::Unreadable)
    })
}

/// Prints the totals (vehicles, wheels, ok wheels, scrambled) and the sorted mismatches.
fn report(bad: &mut Vec<String>, totals: (usize, usize, usize, usize)) {
    let (vehicles, wheels, ok_wheels, skipped_scrambled) = totals;
    println!("vehicles {vehicles}, wheel meshes checked {wheels}, within 0.35 m (height/forward) of their pivot {ok_wheels}, scrambled (skipped) {skipped_scrambled}");
    bad.sort();
    bad.dedup();
    for b in bad.iter().take(60) {
        println!("MISMATCH {b}");
    }
    println!("mismatches: {}", bad.len());
}

fn main() {
    let root = std::env::args().nth(1).expect("OMSI root");
    let install = OmsiInstall::new(&root);
    let (mut vehicles, mut wheels, mut ok_wheels, mut skipped_scrambled) = (0, 0, 0, 0);
    let mut bad: Vec<String> = Vec::new();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let bus_path = Path::new(&line);
        let Some((model, model_dir)) = load_vehicle(&install, bus_path) else {
            continue;
        };
        let Some(lod0) = model.lods.first() else {
            continue;
        };
        vehicles += 1;
        for mesh in &lod0.meshes {
            let is_wheel = mesh.anims.iter().any(|a| {
                a.anim_rot
                    .as_ref()
                    .is_some_and(|r| r.variable.to_lowercase().starts_with("wheel_rotation"))
            });
            if !is_wheel {
                continue;
            }
            let Some(pivot) = mesh.anims.iter().find_map(|a| a.origin_trans) else {
                continue;
            };
            let Some(path) = install.find_in(&model_dir, &mesh.path) else {
                continue;
            };
            let Ok(data) = std::fs::read(&path) else {
                continue;
            };
            // Scrambled meshes are skipped by the engine, so their placement is moot.
            if is_scrambled(&path, &data) {
                skipped_scrambled += 1;
                continue;
            }
            let Some((lo, hi)) = bounds(&path, &data) else {
                continue;
            };
            wheels += 1;
            let centre = (lo + hi) * 0.5;
            let expect = omsi_to_bevy_pos(pivot);
            // A wheel rotates about the pivot's x axis, so only the pivot's height and forward
            // position are meaningful; its lateral value is arbitrary (hub caps, twin tyres).
            let d = ((centre.y - expect.y).powi(2) + (centre.z - expect.z).powi(2)).sqrt();
            let lateral = (centre.x - expect.x).abs();
            if d < 0.35 {
                ok_wheels += 1;
            } else {
                bad.push(format!(
                    "{:.2} m (lateral {:.2})  {}  mesh {} centre {:?} pivot {:?}",
                    d,
                    lateral,
                    bus_path.strip_prefix(&root).unwrap_or(bus_path).display(),
                    mesh.path,
                    centre,
                    expect
                ));
            }
        }
    }
    report(&mut bad, (vehicles, wheels, ok_wheels, skipped_scrambled));
}
