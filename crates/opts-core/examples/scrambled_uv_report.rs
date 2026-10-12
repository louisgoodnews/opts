//! How well the confident decoder restores the meshes of one vehicle model (P-1012): per mesh the
//! acceptance, the vertex agreement and the texture-jitter outcome (`clean`, `corrected`,
//! `unresolved` vertices: unresolved ones may still sample the wrong part of their texture).
//! Usage: `scrambled_uv_report <model.cfg> [name filter]`
use std::path::{Path, PathBuf};

use opts_core::formats::o3d::descramble::decode_confident;
use opts_core::formats::o3d::parse;
use opts_core::formats::vehicles::model::ModelConfig;

fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for c in rel.split(['\\', '/']).filter(|c| !c.is_empty()) {
        let hit = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(c))?;
        cur = hit.path();
    }
    Some(cur)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let cfg = PathBuf::from(args.next().expect("model .cfg"));
    let filter = args.next().map(|f| f.to_lowercase());
    let dir = cfg.parent().unwrap().to_path_buf();
    let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).unwrap()).unwrap();
    let lod = model.detail_lod().unwrap();
    let (mut n, mut accepted, mut v_total, mut v_clean, mut v_fixed, mut v_unres) = (0, 0, 0usize, 0usize, 0usize, 0usize);
    let mut seen = std::collections::HashSet::new();
    println!("mesh | vertices | accepted | agreement | uv clean/corrected/unresolved");
    for m in &lod.meshes {
        if filter.as_ref().is_some_and(|f| !m.path.to_lowercase().contains(f)) || !seen.insert(m.path.to_lowercase()) {
            continue;
        }
        let Some(path) = find_ci(&dir, &m.path) else { continue };
        let Ok(mut o) = parse(&std::fs::read(path).unwrap()) else { continue };
        if o.encryption_key.is_none() {
            continue;
        }
        let r = decode_confident(&mut o);
        n += 1;
        accepted += usize::from(r.accepted);
        if r.accepted {
            v_total += o.vertices.len();
            v_clean += r.uv.clean;
            v_fixed += r.uv.corrected;
            v_unres += r.uv.unresolved;
        }
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for v in &o.vertices {
            for (k, c) in [v.uv.x, v.uv.y].into_iter().enumerate() {
                lo[k] = lo[k].min(c);
                hi[k] = hi[k].max(c);
            }
        }
        if filter.is_some() {
            let mut h: std::collections::BTreeMap<(i32, i32), usize> = Default::default();
            for v in &o.vertices {
                *h.entry(((v.uv.x * 1000.0).round() as i32, (v.uv.y * 1000.0).round() as i32)).or_default() += 1;
            }
            let mut top: Vec<_> = h.iter().collect();
            top.sort_by_key(|e| std::cmp::Reverse(*e.1));
            let (b, _) = top[0];
            println!("  {} distinct uvs; most common (thousandths) and offsets from it:", h.len());
            for (k, c) in top.iter().take(12) {
                println!("    {:?} x{} offset ({:+.3}, {:+.3})", k, c, (k.0 - b.0) as f32 / 1000.0, (k.1 - b.1) as f32 / 1000.0);
            }
            let mut us: Vec<f32> = o.vertices.iter().map(|v| v.uv.x).collect();
            let mut vs: Vec<f32> = o.vertices.iter().map(|v| v.uv.y).collect();
            us.sort_by(f32::total_cmp);
            vs.sort_by(f32::total_cmp);
            let (mu, mv) = (us[us.len() / 2], vs[vs.len() / 2]);
            let far = o.vertices.iter().filter(|v| (v.uv.x - mu).abs() > 0.2 || (v.uv.y - mv).abs() > 0.2).count();
            println!("  median uv ({mu:.3}, {mv:.3}); {far} of {} vertices farther than 0.2 from it", o.vertices.len());
            println!("  uv after decoding: u {:.3}..{:.3} v {:.3}..{:.3}", lo[0], hi[0], lo[1], hi[1]);
        }
        println!(
            "{} | {} | {} | {:.2} | {}/{}/{}",
            m.path,
            o.vertices.len(),
            r.accepted,
            r.agreement,
            r.uv.clean,
            r.uv.corrected,
            r.uv.unresolved
        );
    }
    let pct = |x: usize| 100.0 * x as f64 / v_total.max(1) as f64;
    println!(
        "scrambled meshes {n}, accepted {accepted}; of {v_total} vertices in accepted meshes: clean {:.1}%, corrected {:.1}%, unresolved {:.1}%",
        pct(v_clean),
        pct(v_fixed),
        pct(v_unres)
    );
}
