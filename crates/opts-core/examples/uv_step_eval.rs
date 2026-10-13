//! Scores the UV-step parameters (`UvFixParams`, overridable by `UVFIX_*` environment variables) on
//! the twin pairs of `uv_truth`'s `pairs.tsv`, using the true positions. Usage:
//! `uv_step_eval <pairs.tsv> [max pairs]`.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use opts_core::formats::o3d::parse;
use opts_core::formats::o3d::uvfix::{unjitter_uv_staged, UvFixParams, UvStage};

fn main() {
    let mut args = std::env::args().skip(1);
    let text = std::fs::read_to_string(args.next().unwrap()).unwrap();
    let max: usize = args
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);
    // EVAL_PARITY=odd|even keeps only pairs the chain tables were (not) trained on.
    let parity = std::env::var("EVAL_PARITY").ok();
    let parity_ok = |id: &str| match (parity.as_deref(), id.parse::<u32>()) {
        (Some("odd"), Ok(i)) => i % 2 == 1,
        (Some("even"), Ok(i)) => i % 2 == 0,
        _ => true,
    };
    let pairs: Vec<(String, String)> = text
        .lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() > 7 && f[7] == "true" && parity_ok(f[0]))
                .then(|| (f[1].to_owned(), f[2].to_owned()))
        })
        .take(max)
        .collect();
    let params = UvFixParams::default().with_overrides(|k| std::env::var(k).ok());
    // [vertices, exact, vertices 5|n, exact 5|n, unresolved]
    let acc = Mutex::new([0usize; 5]);
    let energy = Mutex::new((0usize, 0usize, 0usize, 0usize));
    let gap = Mutex::new([0.0f64; 3]);
    let fallback = Mutex::new([0usize; 7]);
    let unrel = Mutex::new([0usize; 7]);
    let qual = Mutex::new([(0usize, 0usize); 3]);
    let calib: Mutex<Vec<(u8, f32, bool)>> = Mutex::new(Vec::new());
    let stage_acc = Mutex::new([(0usize, 0usize); 7]);
    let top = Mutex::new([0usize; 4]);
    let oracle = Mutex::new((0usize, 0usize, 0usize, 0usize, 0usize));
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..16 {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((pp, kp)) = pairs.get(i) else { break };
                if std::env::var_os("EVAL_FIVE_ONLY").is_some() && pp.is_empty() {
                    continue;
                }
                let (Ok(po), Ok(ko)) = (
                    parse(&std::fs::read(pp).unwrap()),
                    parse(&std::fs::read(kp).unwrap()),
                ) else {
                    continue;
                };
                let mut t = ko.clone();
                for (v, truth) in t.vertices.iter_mut().zip(&po.vertices) {
                    v.position = truth.position;
                }
                let kt = t.clone(); // keyed uv, true positions
                let outcome = unjitter_uv_staged(&mut t, &[], &params);
                let (rep, stages) = (outcome.report, outcome.stages.clone());
                let n = po.vertices.len();
                if std::env::var_os("EVAL_FIVE_ONLY").is_some() && n % 5 != 0 {
                    continue;
                }
                {
                    let mut st = stage_acc.lock().unwrap();
                    for v in 0..n {
                        let ok = (t.vertices[v].uv.x - po.vertices[v].uv.x).abs() < 1e-4
                            && (t.vertices[v].uv.y - po.vertices[v].uv.y).abs() < 1e-4;
                        let e = &mut st[stages[v] as usize];
                        e.0 += 1;
                        e.1 += usize::from(ok);
                    }
                }
                {
                    let mut c = calib.lock().unwrap();
                    for v in 0..n {
                        let ok = (t.vertices[v].uv.x - po.vertices[v].uv.x).abs() < 1e-4
                            && (t.vertices[v].uv.y - po.vertices[v].uv.y).abs() < 1e-4;
                        c.push((outcome.stages[v] as u8, outcome.confidence[v], ok));
                        let q = &mut qual.lock().unwrap()[outcome.quality[v].code() as usize];
                        q.0 += 1;
                        q.1 += usize::from(ok);
                    }
                }
                {
                    let uvs: Vec<glam::Vec2> = t.vertices.iter().map(|v| v.uv).collect();
                    let trusted: Vec<bool> = outcome
                        .quality
                        .iter()
                        .map(|q| *q != opts_core::formats::o3d::uvfix::UvQuality::Unreliable)
                        .collect();
                    let fb =
                        opts_core::formats::o3d::uvfix::hinge_fallback(&t, &uvs, &trusted, 5e-3);
                    let mut f = fallback.lock().unwrap();
                    for v in 0..n {
                        if trusted[v] {
                            continue;
                        }
                        f[0] += 1;
                        let truth = po.vertices[v].uv;
                        f[1] += usize::from((uvs[v] - truth).abs().max_element() < 1e-4);
                        f[2] += usize::from((uvs[v] - truth).abs().max_element() < 1e-2);
                        if let Some(p) = fb[v] {
                            f[3] += 1;
                            f[4] += usize::from((p - truth).abs().max_element() < 1e-4);
                            f[5] += usize::from((p - truth).abs().max_element() < 1e-2);
                            f[6] += usize::from((uvs[v] - truth).abs().max_element() < 1e-2);
                        }
                    }
                }
                {
                    // What the Unreliable tier gets wrong, by kind (stored = keyed value).
                    let mut u = unrel.lock().unwrap();
                    for v in 0..n {
                        if outcome.quality[v] != opts_core::formats::o3d::uvfix::UvQuality::Unreliable {
                            continue;
                        }
                        let (st, dec, tru) = (ko.vertices[v].uv, t.vertices[v].uv, po.vertices[v].uv);
                        let guess_none = (st - dec).abs().max_element() < 1e-6;
                        let truth_none = (st - tru).abs().max_element() < 1e-6;
                        let right = (dec - tru).abs().max_element() < 1e-4;
                        u[0] += 1;
                        u[1] += usize::from(right);
                        u[2] += usize::from(truth_none);
                        u[3] += usize::from(guess_none);
                        if !right {
                            u[4] += usize::from(guess_none && !truth_none); // left jittered
                            u[5] += usize::from(!guess_none && truth_none); // shifted a clean vertex
                            u[6] += usize::from(!guess_none && !truth_none); // wrong offset
                        }
                    }
                }
                let exact = (0..n)
                    .filter(|&v| {
                        (t.vertices[v].uv.x - po.vertices[v].uv.x).abs() < 1e-4
                            && (t.vertices[v].uv.y - po.vertices[v].uv.y).abs() < 1e-4
                    })
                    .count();
                if std::env::var_os("UV_ORACLE").is_some() {
                    let truth: Vec<glam::Vec2> = po.vertices.iter().map(|v| v.uv).collect();
                    if let Some(c) =
                        opts_core::formats::o3d::uvfix::single_vertex_ceiling(&kt, &truth, &params)
                    {
                        let mut o = oracle.lock().unwrap();
                        for (a, b) in top.lock().unwrap().iter_mut().zip(c.3) {
                            *a += b;
                        }
                        o.0 += c.0;
                        o.1 += c.1;
                        o.2 += c.2;
                        if n % 5 == 0 {
                            o.3 += c.0;
                            o.4 += c.1;
                        }
                    }
                }
                if std::env::var_os("UV_ENERGY").is_some() {
                    let truth: Vec<glam::Vec2> = po.vertices.iter().map(|v| v.uv).collect();
                    let decoded: Vec<glam::Vec2> = t.vertices.iter().map(|v| v.uv).collect();
                    let et =
                        opts_core::formats::o3d::uvfix::assignment_energy(&kt, &truth, &params);
                    let ed =
                        opts_core::formats::o3d::uvfix::assignment_energy(&kt, &decoded, &params);
                    if let (Some(et), Some(ed)) = (et, ed) {
                        let (tt, dd) = (et.0 + et.1, ed.0 + ed.1);
                        {
                            let mut g = gap.lock().unwrap();
                            g[0] += f64::from(et.0 - ed.0);
                            g[1] += f64::from(et.1 - ed.1);
                            g[2] += po.vertices.len() as f64;
                        }
                        let mut e = energy.lock().unwrap();
                        e.0 += 1;
                        if tt < dd - 1e-3 {
                            e.1 += 1; // truth scores lower: the search missed it
                        } else if (tt - dd).abs() <= 1e-3 {
                            e.2 += 1;
                        } else {
                            e.3 += 1; // decoded scores lower than the truth: the model prefers a wrong answer
                        }
                    }
                }
                let mut a = acc.lock().unwrap();
                a[0] += n;
                a[1] += exact;
                if n % 5 == 0 {
                    a[2] += n;
                    a[3] += exact;
                }
                a[4] += rep.unresolved;
            });
        }
    });
    let e = energy.into_inner().unwrap();
    if e.0 > 0 {
        println!("energy check on {} meshes: truth lower (search failure) {}, equal {}, decoded lower (model failure) {}", e.0, e.1, e.2, e.3);
    }
    let u = unrel.into_inner().unwrap();
    println!("unreliable tier: {} vertices, right {:.3}; truly jittered {:.3}; best guess leaves the value unchanged for {:.3}; of the wrong ones: left jittered {:.3}, clean vertex shifted {:.3}, wrong offset {:.3}", u[0], u[1] as f64 / u[0] as f64, 1.0 - u[2] as f64 / u[0] as f64, u[3] as f64 / u[0] as f64, u[4] as f64 / (u[0] - u[1]) as f64, u[5] as f64 / (u[0] - u[1]) as f64, u[6] as f64 / (u[0] - u[1]) as f64);
    let f = fallback.into_inner().unwrap();
    println!("fallback on {} unreliable vertices: best guess exact {:.3}, within 0.01 {:.3}; hinge prediction exists for {} ({:.1}%): exact {:.3}, within 0.01 {:.3} (the guess on the same vertices: within 0.01 {:.3})",
        f[0], f[1] as f64 / f[0] as f64, f[2] as f64 / f[0] as f64, f[3], 100.0 * f[3] as f64 / f[0] as f64, f[4] as f64 / f[3].max(1) as f64, f[5] as f64 / f[3].max(1) as f64, f[6] as f64 / f[3].max(1) as f64);
    let qual = qual.into_inner().unwrap();
    let qtotal: usize = qual.iter().map(|q| q.0).sum();
    for (i, name) in ["Exact", "Recovered", "Unreliable"].iter().enumerate() {
        println!(
            "quality {name:10} {:6.2}% of vertices, precision {:.4}",
            100.0 * qual[i].0 as f64 / qtotal.max(1) as f64,
            qual[i].1 as f64 / qual[i].0.max(1) as f64
        );
    }
    let calib = calib.into_inner().unwrap();
    {
        let total = calib.len() as f64;
        println!("calibration (evidence strength -> precision, share of all vertices):");
        for (stage, name, cuts) in [
            (
                2u8,
                "plane  (anchors)",
                vec![6.0f32, 10.0, 20.0, 40.0, 80.0],
            ),
            (
                3u8,
                "lattice(verified)",
                vec![3.0f32, 4.0, 6.0, 8.0, 12.0, 20.0, 40.0],
            ),
            (
                6u8,
                "smooth (margin) ",
                vec![0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 6.0, 8.0],
            ),
        ] {
            for cut in cuts {
                let sel: Vec<_> = calib
                    .iter()
                    .filter(|e| e.0 == stage && e.1 >= cut)
                    .collect();
                let ok = sel.iter().filter(|e| e.2).count();
                println!(
                    "  {name} >= {cut:5}: {:6.2}% of vertices, precision {:.4}",
                    100.0 * sel.len() as f64 / total,
                    ok as f64 / sel.len().max(1) as f64
                );
            }
        }
    }
    let st = stage_acc.into_inner().unwrap();
    let names = [
        "unresolved",
        "hint",
        "plane",
        "lattice",
        "pool",
        "hinge",
        "smooth",
    ];
    let total: usize = st.iter().map(|e| e.0).sum();
    for (i, e) in st.iter().enumerate() {
        if e.0 > 0 {
            println!(
                "stage {:10} {:6.2}% of vertices, exact {:.4}",
                names[i],
                100.0 * e.0 as f64 / total as f64,
                e.1 as f64 / e.0 as f64
            );
        }
    }
    let _ = UvStage::Hint;
    let top = top.into_inner().unwrap();
    let o = oracle.into_inner().unwrap();
    if o.1 > 0 {
        println!("true offset among the best 1/3/5/10 candidates (others at truth): {:.4} / {:.4} / {:.4} / {:.4}", top[0] as f64 / o.1 as f64, top[1] as f64 / o.1 as f64, top[2] as f64 / o.1 as f64, top[3] as f64 / o.1 as f64);
    }
    if o.1 > 0 {
        println!("single-vertex ceiling (others at truth): {:.4} of {} vertices ({} without any hinge); 5|n: {:.4}", o.0 as f64 / o.1 as f64, o.1, o.2, o.3 as f64 / o.4.max(1) as f64);
    }
    let g = gap.into_inner().unwrap();
    if g[2] > 0.0 {
        println!(
            "mean per vertex (truth minus decoded): hinge energy {:+.4}, prior energy {:+.4}",
            g[0] / g[2],
            g[1] / g[2]
        );
    }
    let a = acc.into_inner().unwrap();
    println!(
        "vertices {} exact {:.4} | 5|n: {} exact {:.4} | other exact {:.4} | unresolved {}",
        a[0],
        a[1] as f64 / a[0] as f64,
        a[2],
        a[3] as f64 / a[2].max(1) as f64,
        (a[1] - a[3]) as f64 / (a[0] - a[2]) as f64,
        a[4]
    );
}
