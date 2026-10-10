//! Evaluates the geometric descrambler on plain/scrambled twins (paths on stdin, alternating
//! plain, scrambled). Prints recovery vs gate statistics.
use opts_core::formats::o3d::descramble::{normal_agreement, solved_positions, triangle_verdicts};
use std::io::BufRead;
fn main() {
    let lines: Vec<String> = std::io::stdin()
        .lock()
        .lines()
        .map_while(Result::ok)
        .collect();
    let mut rows = Vec::new();
    let mut globals: Vec<(f32, usize)> = Vec::new();
    for pair in lines.chunks(2) {
        let (Ok(a), Ok(b)) = (std::fs::read(&pair[0]), std::fs::read(&pair[1])) else {
            continue;
        };
        let (Ok(plain), Ok(enc)) = (
            opts_core::formats::o3d::parse(&a),
            opts_core::formats::o3d::parse(&b),
        ) else {
            continue;
        };
        if plain.vertices.len() != enc.vertices.len()
            || enc.vertices.len() < 12
            || enc.vertices.len() > 20000
        {
            continue;
        }
        let sol = solved_positions(&enc);
        let ok = sol
            .iter()
            .zip(&plain.vertices)
            .filter(|(s, p)| (**s - p.position).abs().max_element() < 1e-4)
            .count();
        let recovery = ok as f32 / sol.len() as f32;
        // best recovery after one global axis permutation (a coherent but re-oriented mesh)
        let perms: [[usize; 3]; 6] = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        let mut best_g = (0.0f32, 0usize);
        for (gi, g) in perms.iter().enumerate() {
            let c = sol
                .iter()
                .zip(&plain.vertices)
                .filter(|(s, p)| {
                    let a = [s.x, s.y, s.z];
                    let t = [p.position.x, p.position.y, p.position.z];
                    (0..3).all(|k| (a[g[k]] - t[k]).abs() < 1e-4)
                })
                .count();
            let f = c as f32 / sol.len() as f32;
            if f > best_g.0 {
                best_g = (f, gi);
            }
        }
        globals.push(best_g);
        let mut trial = enc.clone();
        for (v, s) in trial.vertices.iter_mut().zip(&sol) {
            v.position = *s;
        }
        let (_, coh) = triangle_verdicts(&trial);
        let (_, truth_coh) = triangle_verdicts(&plain);
        let na = normal_agreement(&trial, 0.15);
        let mut truth_mesh = enc.clone();
        for (v, p) in truth_mesh.vertices.iter_mut().zip(&plain.vertices) {
            v.position = p.position;
        }
        let na_truth = normal_agreement(&truth_mesh, 0.15);
        rows.push((recovery, coh, truth_coh, na, na_truth));
    }
    println!("twins evaluated: {}", rows.len());
    let g98 = globals.iter().filter(|g| g.0 >= 0.98).count();
    let g_ident = globals.iter().filter(|g| g.0 >= 0.98 && g.1 == 0).count();
    println!(
        "correct up to ONE global axis permutation (>=98%): {:.3} (of which identity: {:.3})",
        g98 as f32 / globals.len() as f32,
        g_ident as f32 / globals.len() as f32
    );
    let mean: f32 = rows.iter().map(|r| r.0).sum::<f32>() / rows.len() as f32;
    println!(
        "mean recovery {mean:.3}; >=0.98: {:.3}",
        rows.iter().filter(|r| r.0 >= 0.98).count() as f32 / rows.len() as f32
    );
    let mut truth: Vec<f32> = rows.iter().map(|r| r.2).collect();
    truth.sort_by(|a, b| a.total_cmp(b));
    println!(
        "coherence of the TRUE meshes: p5 {:.3} p25 {:.3} median {:.3}",
        truth[truth.len() / 20],
        truth[truth.len() / 4],
        truth[truth.len() / 2]
    );
    let mut t: Vec<f32> = rows.iter().map(|r| r.4).collect();
    t.sort_by(|a, b| a.total_cmp(b));
    println!(
        "normal agreement of the TRUE positions: p5 {:.3} p25 {:.3} median {:.3}",
        t[t.len() / 20],
        t[t.len() / 4],
        t[t.len() / 2]
    );
    for gate in [0.5f32, 0.7, 0.8, 0.9, 0.95, 0.99] {
        let acc: Vec<_> = rows.iter().filter(|r| r.3 >= gate).collect();
        let good = acc.iter().filter(|r| r.0 >= 0.98).count();
        let mean_acc = acc.iter().map(|r| r.0).sum::<f32>() / acc.len().max(1) as f32;
        println!("normal-agreement gate {gate:.2}: accepted {:4} ({:.0}%), fully recovered {:4} ({:.0}%), mean recovery {:.3}", acc.len(), 100.0 * acc.len() as f32 / rows.len() as f32, good, 100.0 * good as f32 / acc.len().max(1) as f32, mean_acc);
    }
}
