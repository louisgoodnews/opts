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

use glam::Vec3;
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
            if n.ends_with(".o3d") {
                out.push(p);
            } else {
                names.insert(n);
            }
        }
    }
}

fn percentile(v: &mut [f32], q: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f32 * q) as usize]
}

/// Geometric / structural checks on a (possibly decoded) mesh.
fn check(o: &O3d, tag: &str, codes: &mut Vec<String>, metrics: &mut Vec<String>) {
    let nv = o.vertices.len();
    let nt = o.indices.len();
    let finite = |v: Vec3| v.is_finite();
    if o.vertices
        .iter()
        .any(|v| !finite(v.position) || !finite(v.normal) || !v.uv.is_finite())
    {
        codes.push(format!("{tag}NONFINITE"));
    }
    if o.indices.iter().any(|t| t.iter().any(|&i| i as usize >= nv)) {
        codes.push(format!("{tag}INDEX_OOR"));
        return;
    }
    if o.face_materials.len() != nt {
        codes.push(format!("{tag}FACEMAT_LEN"));
    }
    if !o.materials.is_empty() && o.face_materials.iter().any(|&m| m as usize >= o.materials.len())
    {
        codes.push(format!("{tag}MATIDX_OOR"));
    }
    if nv == 0 || nt == 0 {
        codes.push(format!("{tag}EMPTY"));
        return;
    }
    // bounding box and outliers
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for v in &o.vertices {
        if finite(v.position) {
            lo = lo.min(v.position);
            hi = hi.max(v.position);
        }
    }
    let diag = (hi - lo).length();
    metrics.push(format!("{tag}diag={diag:.1}"));
    if diag > 3000.0 {
        codes.push(format!("{tag}HUGE_BBOX"));
    }
    let mut med = [0.0f32; 3];
    for (k, m) in med.iter_mut().enumerate() {
        let mut c: Vec<f32> = o
            .vertices
            .iter()
            .map(|v| v.position[k])
            .filter(|x| x.is_finite())
            .collect();
        *m = percentile(&mut c, 0.5);
    }
    let med = Vec3::from(med);
    let d: Vec<f32> = o
        .vertices
        .iter()
        .map(|v| (v.position - med).length())
        .filter(|x| x.is_finite())
        .collect();
    let r90 = percentile(&mut d.clone(), 0.9);
    let out = d.iter().filter(|&&x| x > r90 * 8.0 + 5.0).count();
    if out > 0 {
        metrics.push(format!("{tag}outliers={out}/{nv}"));
        if out * 100 >= nv.max(20) || out >= 3 {
            codes.push(format!("{tag}OUTLIER_VERTS"));
        }
    }
    // triangles
    let (mut degen, mut huge, mut flip, mut agree_n) = (0usize, 0usize, 0usize, 0usize);
    let mut edges: Vec<f32> = Vec::with_capacity(nt);
    let mut uv_ratio: Vec<f32> = Vec::with_capacity(nt);
    let mut tri_edges = Vec::with_capacity(nt);
    for t in &o.indices {
        let [a, b, c] = t.map(|i| &o.vertices[i as usize]);
        let (e1, e2) = (b.position - a.position, c.position - a.position);
        let area = e1.cross(e2).length();
        if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] || area < 1e-9 {
            degen += 1;
            continue;
        }
        let maxe = e1.length().max(e2.length()).max((c.position - b.position).length());
        edges.push(maxe);
        tri_edges.push(maxe);
        let g = e1.cross(e2) / area;
        let vn = a.normal + b.normal + c.normal;
        if vn.length() > 1e-3 {
            let cs = g.dot(vn.normalize());
            if cs.abs() > 0.5 {
                agree_n += 1;
                if cs < 0.0 {
                    flip += 1;
                }
            }
        }
        let uvm = (b.uv - a.uv).length().max((c.uv - a.uv).length()).max((c.uv - b.uv).length());
        if maxe > 1e-4 {
            uv_ratio.push(uvm / maxe);
        }
    }
    if degen * 20 > nt {
        codes.push(format!("{tag}DEGENERATE_TRIS"));
    }
    if degen > 0 {
        metrics.push(format!("{tag}degen={degen}/{nt}"));
    }
    let med_edge = percentile(&mut edges.clone(), 0.5);
    for &e in &tri_edges {
        if e > med_edge * 25.0 && e > 100.0 {
            huge += 1;
        }
    }
    if huge > 0 {
        metrics.push(format!("{tag}huge_tris={huge}/{nt}"));
        if huge * 50 > nt || huge >= 3 {
            codes.push(format!("{tag}HUGE_TRIS"));
        }
    }
    if agree_n >= 8 {
        let f = flip as f32 / agree_n as f32;
        metrics.push(format!("{tag}flip={f:.2}"));
        if (0.25..0.75).contains(&f) {
            codes.push(format!("{tag}WINDING_MIXED"));
        }
    }
    // uv chaos
    if uv_ratio.len() >= 8 {
        let m = percentile(&mut uv_ratio.clone(), 0.5).max(1e-6);
        let bad = uv_ratio.iter().filter(|&&r| r > m * 40.0 && r > 2.0).count();
        let frac = bad as f32 / uv_ratio.len() as f32;
        metrics.push(format!("{tag}uvchaos={frac:.2}"));
        if frac > 0.10 {
            codes.push(format!("{tag}UV_CHAOS"));
        }
    }
    if o.vertices.iter().any(|v| v.uv.abs().max_element() > 1000.0) {
        codes.push(format!("{tag}UV_HUGE"));
    }
}

fn audit(path: &Path, textures: &HashSet<String>, decode: bool) -> (String, String, String) {
    let mut codes: Vec<String> = Vec::new();
    let mut metrics: Vec<String> = Vec::new();
    let rel = path.to_string_lossy().into_owned();
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return (rel, "UNREADABLE".into(), String::new()),
    };
    let o = match parse(&bytes) {
        Ok(o) => o,
        Err(e) => {
            let s = e.to_string();
            let class = s.split([':', ' ']).take(2).collect::<Vec<_>>().join("_");
            return (rel, format!("PARSE_ERROR({class})"), s);
        }
    };
    metrics.push(format!(
        "v{} nv={} nt={} nm={}",
        o.version,
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
        let r = decode_confident(&mut d);
        metrics.push(format!(
            "dec_acc={} agree={:.2} dropped={} uv={}/{}/{}",
            r.accepted, r.agreement, r.dropped_faces, r.uv.clean, r.uv.corrected, r.uv.unresolved
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
                let r = std::panic::catch_unwind(|| audit(p, &names, decode)).unwrap_or_else(|_| {
                    (p.to_string_lossy().into_owned(), "PANIC".into(), String::new())
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
