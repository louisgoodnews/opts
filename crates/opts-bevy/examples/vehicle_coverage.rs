//! Per vehicle: how many visible lod0 meshes load, are scrambled (skipped), or fail.
//! Usage: `... | cargo run --release -p opts-bevy --example vehicle_coverage -- <OMSI root>`.
use opts_bevy::{mesh::parse_mesh_file, OmsiInstall};
use opts_core::formats::o3d::VertexLayout;
use opts_core::formats::vehicles::{bus::BusManifest, model::ModelConfig};
use std::io::BufRead;
use std::path::Path;

fn main() {
    let root = std::env::args().nth(1).expect("OMSI root");
    let install = OmsiInstall::new(&root);
    let mut rows = Vec::new();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        let bus_path = Path::new(&line);
        let Ok(bytes) = std::fs::read(bus_path) else {
            continue;
        };
        let Ok(bus) = BusManifest::parse_bytes(&bytes) else {
            continue;
        };
        let Some(cfg_rel) = bus.model_cfg.clone() else {
            continue;
        };
        let Some(cfg) = install.find_in(bus_path.parent().unwrap_or(Path::new(".")), &cfg_rel)
        else {
            continue;
        };
        let Ok(model) = ModelConfig::parse_bytes(&std::fs::read(&cfg).unwrap_or_default()) else {
            rows.push((0.0, 0, 0, 0, 0, format!("{} (model.cfg unparsable)", line)));
            continue;
        };
        let Some(lod0) = model.lods.first() else {
            continue;
        };
        let dir = cfg.parent().unwrap_or(Path::new("."));
        let (mut ok, mut scr, mut missing, mut bad, mut verts_ok, mut verts_scr) =
            (0, 0, 0, 0, 0usize, 0usize);
        for m in &lod0.meshes {
            if m.isshadow || !(m.viewpoint.is_outside() || m.viewpoint.is_inside()) {
                continue;
            }
            if m.visibility.iter().any(|v| v.expected_value != 0.0) {
                continue;
            }
            let Some(p) = install.find_in(dir, &m.path) else {
                missing += 1;
                continue;
            };
            let Ok(data) = std::fs::read(&p) else {
                missing += 1;
                continue;
            };
            match parse_mesh_file(&p, &data) {
                Ok(models) => {
                    let v: usize = models.iter().map(|o| o.vertices.len()).sum();
                    if models
                        .iter()
                        .any(|o| o.vertex_layout() == VertexLayout::Unreadable)
                    {
                        scr += 1;
                        verts_scr += v;
                    } else {
                        ok += 1;
                        verts_ok += v;
                    }
                }
                Err(_) => bad += 1,
            }
        }
        let total = ok + scr + missing + bad;
        let vert_share = verts_ok as f32 / (verts_ok + verts_scr).max(1) as f32;
        rows.push((
            vert_share,
            ok,
            scr,
            missing + bad,
            total,
            line.rsplit("OMSI 2/").next().unwrap_or(&line).to_string(),
        ));
    }
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let n = rows.len();
    let broken = rows.iter().filter(|r| r.0 < 0.5).count();
    println!("vehicles {n}; less than half of their geometry (by vertices) loadable: {broken}");
    for (share, ok, scr, miss, total, name) in rows
        .iter()
        .filter(|r| r.0 < 0.9 && (std::env::var("COV_FILTER").map_or(true, |f| r.5.contains(&f))))
    {
        println!(
            "{:5.1}%  ok {ok:3} scrambled {scr:3} missing/bad {miss:3} of {total:3}  {name}",
            share * 100.0
        );
    }
}
