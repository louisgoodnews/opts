//! Offline statistical audit of every `.o3d` under an OMSI install (P-1014).
//!
//! Per mesh it reports structural and geometric irregularities that make a mesh look "scrambled"
//! on screen: parse errors, non-finite values, index / material-index range, degenerate and
//! huge triangles, exploded (outlier) vertices, bounding-box extent, winding disagreement with the
//! stored normals, texture-coordinate chaos, unresolved texture names and the scrambling state
//! (keyed file, decoded or skipped). Output is one TSV line per mesh: `path<TAB>codes<TAB>metrics`.
//!
//! Usage: `mesh_audit <omsi root> [--decode] [--threads N] [--only SUBSTR]`
//! `--decode` additionally runs the confident decoder on every keyed mesh and audits the decoded
//! result (slow: ~135 ms per mesh, spread over the threads).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use opts_core::formats::o3d::audit::check;
use opts_core::formats::o3d::descramble::decode_confident;
use opts_core::formats::o3d::{parse, O3d, VertexLayout};

fn walk(dir: &Path, out: &mut Vec<PathBuf>, names: &mut HashSet<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            walk(&p, out, names);
        } else {
            let n = e.file_name().to_string_lossy().to_lowercase();
            if n.ends_with(".o3d") || n.ends_with(".x") {
                out.push(p);
            } else {
                names.insert(n);
            }
        }
    }
}

/// Flattens a DirectX `.x` file into [`O3d`] sub-meshes (frame transforms are not applied: the
/// audit looks at the geometry of each sub-mesh).
fn x_submeshes(bytes: &[u8]) -> Result<Vec<O3d>, String> {
    use opts_core::formats::x::{parse_x, XFrame, XSubMesh};
    fn conv(sub: &XSubMesh, global: &[opts_core::formats::x::XMaterial]) -> O3d {
        let palette = if sub.materials.is_empty() {
            global
        } else {
            &sub.materials
        };
        O3d {
            version: 0,
            header_flags: 0,
            vertices: sub
                .vertices
                .iter()
                .map(|v| opts_core::formats::o3d::Vertex {
                    position: v.position,
                    normal: v.normal,
                    uv: v.uv,
                    color: None,
                })
                .collect(),
            indices: sub.indices.clone(),
            face_materials: sub.face_materials.iter().map(|&m| m as u16).collect(),
            materials: palette
                .iter()
                .map(|m| opts_core::formats::o3d::Material {
                    diffuse: m.diffuse,
                    specular: m.specular,
                    emissive: m.emissive,
                    power: m.power,
                    textures: m.texture_filename.clone().into_iter().collect(),
                })
                .collect(),
            transform: None,
            unknown_chunks: Vec::new(),
            encryption_key: None,
            descrambled: false,
        }
    }
    fn walk_frame(f: &XFrame, global: &[opts_core::formats::x::XMaterial], out: &mut Vec<O3d>) {
        out.extend(f.meshes.iter().map(|m| conv(m, global)));
        for c in &f.children {
            walk_frame(c, global, out);
        }
    }
    let x = parse_x(bytes).map_err(|e| e.to_string())?;
    let mut out: Vec<O3d> = x
        .top_level_meshes
        .iter()
        .map(|m| conv(m, &x.global_materials))
        .collect();
    for f in &x.frames {
        walk_frame(f, &x.global_materials, &mut out);
    }
    Ok(out)
}

fn audit(path: &Path, textures: &HashSet<String>, decode: bool) -> (String, String, String) {
    let mut codes: Vec<String> = Vec::new();
    let mut metrics: Vec<String> = Vec::new();
    let rel = path.to_string_lossy().into_owned();
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return (rel, "UNREADABLE".into(), String::new()),
    };
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("x"))
    {
        let subs = match x_submeshes(&bytes) {
            Ok(s) if !s.is_empty() => s,
            Ok(_) => return (rel, "EMPTY".into(), "x".into()),
            Err(e) => return (rel, "PARSE_ERROR(x)".into(), e),
        };
        metrics.push(format!("x subs={}", subs.len()));
        for o in &subs {
            check(o, "", &mut codes, &mut metrics);
        }
        codes.sort();
        codes.dedup();
        return (rel, codes.join(","), metrics.join(" "));
    }
    let o = match parse(&bytes) {
        Ok(o) => o,
        Err(e) => {
            let s = e.to_string();
            let class = s.split([':', ' ']).take(2).collect::<Vec<_>>().join("_");
            return (rel, format!("PARSE_ERROR({class})"), s);
        }
    };
    metrics.push(format!(
        "v{} {:?} nv={} nt={} nm={}",
        o.version,
        o.vertex_layout(),
        o.vertices.len(),
        o.indices.len(),
        o.materials.len()
    ));
    let keyed = o.encryption_key.is_some();
    let layout = o.vertex_layout();
    if keyed {
        codes.push("KEYED".into());
    }
    if layout == VertexLayout::Unreadable {
        codes.push("SCRAMBLED_SKIPPED".into());
    }
    // texture binding
    for m in &o.materials {
        for t in &m.textures {
            let base = t.rsplit(['\\', '/']).next().unwrap_or(t).to_lowercase();
            if base.is_empty() {
                continue;
            }
            let stem = base.rsplit_once('.').map_or(base.as_str(), |(s, _)| s);
            let found = textures.contains(&base)
                || ["bmp", "dds", "tga", "jpg", "png", "jpeg"]
                    .iter()
                    .any(|e| textures.contains(&format!("{stem}.{e}")));
            if !found {
                codes.push("TEX_MISSING".into());
                metrics.push(format!("tex={base}"));
                break;
            }
        }
    }
    if o.materials.is_empty() && !o.indices.is_empty() {
        codes.push("NO_MATERIALS".into());
    }
    if !(keyed && layout == VertexLayout::Unreadable) {
        check(&o, "", &mut codes, &mut metrics);
    }
    if decode && keyed {
        let mut d = o.clone();
        let t0 = std::time::Instant::now();
        let r = decode_confident(&mut d);
        let ms = t0.elapsed().as_millis();
        metrics.push(format!(
            "dec_acc={} agree={:.2} dropped={} uv={}/{}/{} dec_ms={}",
            r.accepted,
            r.agreement,
            r.dropped_faces,
            r.uv.clean,
            r.uv.corrected,
            r.uv.unresolved,
            ms
        ));
        if r.accepted {
            codes.push("DECODED".into());
            if r.dropped_faces > 0 {
                codes.push("DECODED_DROPPED_FACES".into());
            }
            if r.uv.unresolved * 10 > d.vertices.len() {
                codes.push("DECODED_UV_UNRESOLVED".into());
            }
            check(&d, "dec_", &mut codes, &mut metrics);
        } else {
            codes.push("DECODE_REJECTED".into());
        }
    }

    (rel, codes.join(","), metrics.join(" "))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("omsi root"));
    let (mut decode, mut threads, mut only) = (false, 8usize, None::<String>);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--decode" => decode = true,
            "--threads" => threads = args.next().and_then(|s| s.parse().ok()).unwrap_or(8),
            "--only" => only = args.next().map(|s| s.to_lowercase()),
            _ => {}
        }
    }
    let (mut files, mut names) = (Vec::new(), HashSet::new());
    walk(&root, &mut files, &mut names);
    files.sort();
    if let Some(f) = &only {
        files.retain(|p| p.to_string_lossy().to_lowercase().contains(f));
    }
    eprintln!("{} o3d files, {} other files", files.len(), names.len());
    let next = AtomicUsize::new(0);
    let out = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(p) = files.get(i) else { break };
                let r =
                    std::panic::catch_unwind(|| audit(p, &names, decode)).unwrap_or_else(|_| {
                        (
                            p.to_string_lossy().into_owned(),
                            "PANIC".into(),
                            String::new(),
                        )
                    });
                out.lock().unwrap().push(r);
                if i % 5000 == 4999 {
                    eprintln!("{} done", i + 1);
                }
            });
        }
    });
    let mut rows = out.into_inner().unwrap();
    rows.sort();
    let strip = root.to_string_lossy().into_owned();
    for (p, c, m) in rows {
        println!("{}\t{}\t{}", p.strip_prefix(&strip).unwrap_or(&p), c, m);
    }
}
