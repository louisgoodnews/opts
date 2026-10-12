//! Ground truth for the UV decoder (P-1013): finds every plain/keyed twin pair of `.o3d` files
//! under an OMSI installation, compares texture coordinates vertex by vertex and writes
//!
//! * `<out>/pairs.tsv` - one line per pair (paths, sizes, counts, equal slots/version/chunks),
//! * `<out>/vertices.tsv` - one line per vertex of every pair (index, key, permutation class,
//!   true jitter `(du, dv)`, whether the end-to-end decode and the UV step alone got it right).
//!
//! Usage: `uv_truth <OMSI root> <out dir> [max pairs]`. A pair is two files with the same file name
//! (case-insensitive), vertex and face counts, `same_model` geometry, one plain and one keyed;
//! of several keyed copies the first (sorted path) is used.
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use opts_core::formats::o3d::descramble::{decode_confident_with, same_model, DecodeParams};
use opts_core::formats::o3d::{parse, uvfix, O3d};

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("o3d"))
        {
            out.push(p);
        }
    }
}

const PERMS: [[usize; 3]; 6] = [[0, 1, 2], [2, 1, 0], [1, 0, 2], [0, 2, 1], [1, 2, 0], [2, 0, 1]];

fn perm_class(plain: [f32; 3], stored: [f32; 3]) -> i32 {
    let hits: Vec<usize> = PERMS
        .iter()
        .enumerate()
        .filter(|(_, p)| (0..3).all(|k| plain[k] == stored[p[k]]))
        .map(|(i, _)| i)
        .collect();
    match hits.as_slice() {
        [] => -1,
        [i] => *i as i32,
        _ => -2, // ambiguous: equal coordinates
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("OMSI root"));
    let out_dir = PathBuf::from(args.next().expect("out dir"));
    let max_pairs: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
    std::fs::create_dir_all(&out_dir).unwrap();
    let mut files = Vec::new();
    walk(&root, &mut files);
    files.sort();
    eprintln!("{} .o3d files", files.len());
    // Header-level scan: (name, counts) -> plain / keyed paths. Parsing everything is what the earlier tools did too.
    type Key = (String, usize, usize);
    let groups: Mutex<HashMap<Key, (Vec<PathBuf>, Vec<PathBuf>)>> = Mutex::new(HashMap::new());
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..16 {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(p) = files.get(i) else { break };
                let Ok(b) = std::fs::read(p) else { continue };
                if b.len() < 8 || b[0] != 0x84 || b[1] != 0x19 || b[2] < 4 {
                    continue;
                }
                let Ok(o) = parse(&b) else { continue };
                let key = (
                    p.file_name().unwrap().to_string_lossy().to_lowercase(),
                    o.vertices.len(),
                    o.indices.len(),
                );
                let mut g = groups.lock().unwrap();
                let e = g.entry(key).or_default();
                if o.encryption_key.is_some() {
                    e.1.push(p.clone());
                } else {
                    e.0.push(p.clone());
                }
            });
        }
    });
    let mut pairs: Vec<(PathBuf, PathBuf)> = groups
        .into_inner()
        .unwrap()
        .into_values()
        .filter_map(|(mut p, mut k)| {
            p.sort();
            k.sort();
            Some((p.first()?.clone(), k.first()?.clone()))
        })
        .collect();
    pairs.sort();
    pairs.truncate(max_pairs);
    eprintln!("{} candidate pairs", pairs.len());

    let pairs_out = Mutex::new(std::io::BufWriter::new(
        std::fs::File::create(out_dir.join("pairs.tsv")).unwrap(),
    ));
    let verts_out = Mutex::new(std::io::BufWriter::new(
        std::fs::File::create(out_dir.join("vertices.tsv")).unwrap(),
    ));
    writeln!(pairs_out.lock().unwrap(), "pair\tplain\tkeyed\tplain_bytes\tkeyed_bytes\tvertices\ttriangles\tsame_model\tslots_equal\tversion_plain\tversion_keyed\tflags_equal\tchunks_equal\tkey\tdecoded\tdec_exact\tuvstep_exact").unwrap();
    let params = DecodeParams::default();
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..16 {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((pp, kp)) = pairs.get(i) else { break };
                let (pb, kb) = (std::fs::read(pp).unwrap(), std::fs::read(kp).unwrap());
                let (Ok(po), Ok(ko)) = (parse(&pb), parse(&kb)) else { continue };
                let same = same_model(&po, &ko);
                let n = po.vertices.len();
                let slots_equal = po.materials.len() == ko.materials.len()
                    && po.materials.iter().zip(&ko.materials).all(|(a, b)| a.textures == b.textures)
                    && po.face_materials == ko.face_materials;
                let chunks = |o: &O3d| {
                    let mut t: Vec<u8> = o.unknown_chunks.iter().map(|c| c.tag).collect();
                    t.sort_unstable();
                    (t, o.transform.is_some())
                };
                let chunks_equal = chunks(&po) == chunks(&ko);
                let key = ko.encryption_key.unwrap_or(0);
                let (mut decoded, mut dec_exact, mut step_exact) = (false, 0usize, 0usize);
                let eps = 1e-4_f32;
                let uv_ok = |a: glam::Vec2, b: glam::Vec2| (a.x - b.x).abs() < eps && (a.y - b.y).abs() < eps;
                let mut dec_ok = vec![-1i8; n];
                let mut step_ok = vec![-1i8; n];
                let mut dec_err = vec![[f32::NAN; 2]; n];
                let mut step_err = vec![[f32::NAN; 2]; n];
                if same {
                    // End to end: positions and texture coordinates decoded from the keyed file alone.
                    let mut d = ko.clone();
                    let rep = decode_confident_with(&mut d, &params);
                    decoded = rep.accepted;
                    if decoded && d.vertices.len() == n {
                        for v in 0..n {
                            dec_ok[v] = i8::from(uv_ok(d.vertices[v].uv, po.vertices[v].uv));
                            dec_err[v] = [d.vertices[v].uv.x - po.vertices[v].uv.x, d.vertices[v].uv.y - po.vertices[v].uv.y];
                        }
                        dec_exact = dec_ok.iter().filter(|&&x| x == 1).count();
                    }
                    // UV step alone, with the true positions (isolates `uvfix`).
                    let mut t = ko.clone();
                    for (v, truth) in t.vertices.iter_mut().zip(&po.vertices) {
                        v.position = truth.position;
                    }
                    uvfix::unjitter_uv(&mut t);
                    for v in 0..n {
                        step_ok[v] = i8::from(uv_ok(t.vertices[v].uv, po.vertices[v].uv));
                        step_err[v] = [t.vertices[v].uv.x - po.vertices[v].uv.x, t.vertices[v].uv.y - po.vertices[v].uv.y];
                    }
                    step_exact = step_ok.iter().filter(|&&x| x == 1).count();
                }
                // Per-vertex truth.
                let mut slot_of = vec![-1i32; n];
                if same {
                    for (f, tri) in po.indices.iter().enumerate() {
                        for &vi in tri {
                            if let Some(s) = slot_of.get_mut(vi as usize) {
                                if *s < 0 {
                                    *s = i32::from(po.face_materials.get(f).copied().unwrap_or(0));
                                }
                            }
                        }
                    }
                    let mut w = verts_out.lock().unwrap();
                    for v in 0..n {
                        let (a, b) = (&po.vertices[v], &ko.vertices[v]);
                        let perm = perm_class(
                            [a.position.x, a.position.y, a.position.z],
                            [b.position.x, b.position.y, b.position.z],
                        );
                        writeln!(
                            w,
                            "{i}\t{v}\t{n}\t{key}\t{}\t{}\t{perm}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{slot_of_v}\t{:08x}\t{:08x}\t{:.6}\t{:.6}\t{:.6}\t{:.6}",
                            ko.version,
                            ko.header_flags,
                            dec_ok[v],
                            step_ok[v],
                            b.uv.x - a.uv.x,
                            b.uv.y - a.uv.y,
                            a.uv.x,
                            a.uv.y,
                            b.uv.x.to_bits(),
                            b.uv.y.to_bits(),
                            step_err[v][0],
                            step_err[v][1],
                            dec_err[v][0],
                            dec_err[v][1],
                            slot_of_v = slot_of[v]
                        )
                        .unwrap();
                    }
                }
                writeln!(
                    pairs_out.lock().unwrap(),
                    "{i}\t{}\t{}\t{}\t{}\t{n}\t{}\t{same}\t{slots_equal}\t{}\t{}\t{}\t{chunks_equal}\t{key:#x}\t{decoded}\t{dec_exact}\t{step_exact}",
                    pp.display(),
                    kp.display(),
                    pb.len(),
                    kb.len(),
                    po.indices.len(),
                    po.version,
                    ko.version,
                    po.header_flags == ko.header_flags
                )
                .unwrap();
            });
        }
    });
    pairs_out.lock().unwrap().flush().unwrap();
    verts_out.lock().unwrap().flush().unwrap();
}
