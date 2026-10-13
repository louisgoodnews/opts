#![allow(clippy::all, unused)]
//! UV decoder contract (P-1013): exactness on a synthetic keyed mesh, and on real plain/keyed twin
//! pairs of an OMSI installation when one is available (`OPTS_OMSI_PATH`, default
//! `/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2`); those tests print a notice and pass
//! without an installation. The meshes are read at run time, nothing is copied.
//!
//! The real-pair thresholds sit between the pre-fix decoder and the fixed one on this fixed sample of
//! 24 pairs (half of its vertices in meshes with `5 | n`, so harder than the whole set: 58 % / 30 % before,
//! 68 % / 47 % after); `UVFIX_*` overrides let a run reproduce the old behaviour, which fails them.

use std::path::{Path, PathBuf};

use glam::{Vec2, Vec3};
use opts_core::formats::o3d::uvfix::{jitter_offsets, unjitter_uv_staged, UvFixParams, UvQuality};
use opts_core::formats::o3d::{parse, O3d, Vertex};

fn params() -> UvFixParams {
    UvFixParams::default().with_overrides(|k| std::env::var(k).ok())
}

/// A flat grid of `w x h` vertices with an affine texture map, as a plain mesh.
fn grid(w: usize, h: usize) -> O3d {
    let mut vertices = Vec::new();
    for j in 0..h {
        for i in 0..w {
            vertices.push(Vertex {
                position: Vec3::new(i as f32 * 0.5, j as f32 * 0.4, 0.0),
                normal: Vec3::Z,
                uv: Vec2::new(0.10 + 0.031 * i as f32, 0.20 + 0.047 * j as f32),
                color: None,
            });
        }
    }
    let mut indices = Vec::new();
    for j in 0..h - 1 {
        for i in 0..w - 1 {
            let a = (j * w + i) as u32;
            let (b, c, d) = (a + 1, a + w as u32, a + w as u32 + 1);
            indices.push([a, b, c]);
            indices.push([b, d, c]);
        }
    }
    O3d {
        version: 5,
        header_flags: 0,
        face_materials: vec![0; indices.len()],
        vertices,
        indices,
        materials: Vec::new(),
        transform: None,
        encryption_key: None,
        descrambled: false,
        unknown_chunks: Vec::new(),
    }
}

/// Adds the scrambler's offsets: `share` of the vertices get one, drawn from the allowed set with a
/// fixed generator (u-only offsets when `5 | n`, as such files carry them).
fn jitter(plain: &O3d, share: u32, seed: u32) -> (O3d, Vec<usize>) {
    let offsets = jitter_offsets();
    let n = plain.vertices.len();
    let allowed: Vec<usize> = if n % 5 == 0 {
        (1..39).collect() // u-only and u+v offsets
    } else {
        (1..offsets.len()).collect()
    };
    let mut state = seed;
    let mut next = move || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        state >> 8
    };
    let mut keyed = plain.clone();
    keyed.encryption_key = Some(0x31B6);
    let mut used = vec![0; n];
    for (i, v) in keyed.vertices.iter_mut().enumerate() {
        if next() % 100 < share {
            let k = allowed[next() as usize % allowed.len()];
            v.uv += Vec2::new(offsets[k].0, offsets[k].1);
            used[i] = k;
        }
    }
    (keyed, used)
}

fn exact(a: &O3d, b: &O3d) -> usize {
    a.vertices
        .iter()
        .zip(&b.vertices)
        .filter(|(x, y)| (x.uv - y.uv).abs().max_element() < 1e-4)
        .count()
}

#[test]
fn synthetic_mesh_with_most_vertices_jittered_is_recovered() {
    // 30 vertices (5 | n): 77 % of them carry an offset, as in such files; almost no clean anchors.
    let plain = grid(6, 5);
    let (mut keyed, _) = jitter(&plain, 77, 7);
    let out = unjitter_uv_staged(&mut keyed, &[], &params());
    let right = exact(&keyed, &plain);
    assert!(right >= 28, "{right} of 30 vertices exact");
    assert_eq!(out.quality.len(), 30);
}

#[test]
fn every_vertex_gets_a_quality_and_exact_ones_are_right() {
    let plain = grid(8, 7); // 56 vertices
    let (mut keyed, _) = jitter(&plain, 50, 3);
    let out = unjitter_uv_staged(&mut keyed, &[], &params());
    assert_eq!(out.quality.len(), 56);
    for (i, q) in out.quality.iter().enumerate() {
        if *q == UvQuality::Exact {
            assert!(
                (keyed.vertices[i].uv - plain.vertices[i].uv)
                    .abs()
                    .max_element()
                    < 1e-4,
                "vertex {i} is Exact but wrong"
            );
        }
    }
}

fn omsi_root() -> Option<PathBuf> {
    let p = std::env::var("OPTS_OMSI_PATH").map_or_else(
        |_| PathBuf::from("/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2"),
        PathBuf::from,
    );
    p.is_dir().then_some(p)
}

/// Twin pairs (plain file, keyed file) of the installation, relative to its root.
const PAIRS: &[(&str, &str)] = &[
    (
        "Sceneryobjects/HafenCityHamburg/model/jbp_kontor.o3d",
        "Sceneryobjects/HamburgLinie109/model/jbp_kontor.o3d",
    ),
    (
        "Sceneryobjects/HafenCityHamburgObjects/model/ADAM3.o3d",
        "Sceneryobjects/HamburgLi20Objects/model/ADAM3.o3d",
    ),
    (
        "Vehicles/HH20_HHA_DT4/model/DT4.5_1/DT4.5_Innen_F.o3d",
        "Vehicles/HC_HHA_DT4/model/DT4.5_1/DT4.5_Innen_F.o3d",
    ),
    (
        "Sceneryobjects/HafenCityHamburgObjects/model/ADFM3.o3d",
        "Sceneryobjects/HamburgLi20Objects/model/ADFM3.o3d",
    ),
    (
        "Sceneryobjects/HafenCityHamburg/model/mbs_ew_karstadt.o3d",
        "Sceneryobjects/HamburgLinie109/model/mbs_ew_karstadt.o3d",
    ),
    (
        "Vehicles/HH20_HHA_DT4/model/DT4.5_1/DT4.5_Schaku.o3d",
        "Vehicles/HC_HHA_DT4/model/DT4.5_1/DT4.5_Schaku.o3d",
    ),
    (
        "Sceneryobjects/HafenCityHamburg/model/kia_background.o3d",
        "Sceneryobjects/HamburgLinie109/model/kia_background.o3d",
    ),
    (
        "Sceneryobjects/HafenCityHamburg/model/kns_ew_2+1_1.o3d",
        "Sceneryobjects/HamburgLinie109/model/kns_ew_2+1_1.o3d",
    ),
    (
        "Vehicles/HH_EBus2019/Model/19_aussen_weich2_#low.o3d",
        "Vehicles/HC_EBus2019_parked/Model/19_aussen_weich2_#low.o3d",
    ),
    (
        "Vehicles/HH20_Volvo7900H/Model/V18_scheiben_innen_wisch.o3d",
        "Vehicles/HC_Volvo7900H/Model/V18_scheiben_innen_wisch.o3d",
    ),
    (
        "Vehicles/HH20_Volvo7900H/Model/V18_tuer1.o3d",
        "Vehicles/HC_Volvo7900H/Model/V18_tuer1.o3d",
    ),
    (
        "Sceneryobjects/HafenCityHamburg/model/kia_52-57.o3d",
        "Sceneryobjects/HamburgLinie109/model/kia_52-57.o3d",
    ),
    (
        "Sceneryobjects/HafenCityHamburg/model/bd_ecke.o3d",
        "Sceneryobjects/HamburgLinie109/model/bd_ecke.o3d",
    ),
    (
        "Vehicles/HH20_HHA_DT4/model/DT4.5_2/DT4.5_Fenster_A.o3d",
        "Vehicles/HC_HHA_DT4/model/DT4.5_2/DT4.5_Fenster_A.o3d",
    ),
    (
        "Vehicles/HH20_HHA_DT4/model/DT4.3_2/DT4.3_Fenster_A.o3d",
        "Vehicles/HC_HHA_DT4/model/DT4.3_2/DT4.3_Fenster_A.o3d",
    ),
    (
        "Vehicles/HH20_C2_parked/Model/NL_fensaus.o3d",
        "Vehicles/Gladbeck_Stadtbus/Model/NL_fensaus.o3d",
    ),
    (
        "Vehicles/HH20_C2_parked/Model/VW_fensaus_parked.o3d",
        "Vehicles/HC_C2_parked/Model/VW_fensaus_parked.o3d",
    ),
    (
        "Vehicles/HH20_C2_parked/Model/VW_fensin.o3d",
        "Vehicles/HC_C2_parked/Model/VW_fensin.o3d",
    ),
    (
        "Vehicles/HH20_C2_parked/Model/C2_rad_HL.o3d",
        "Vehicles/HC_C2_parked/Model/C2_rad_HL.o3d",
    ),
    (
        "Vehicles/HH20_Volvo7900H/Model/V18_T_tuer4.o3d",
        "Vehicles/HC_Volvo7900H/Model/V18_T_tuer4.o3d",
    ),
    (
        "Sceneryobjects/HafenCityHamburg/model/2_K_jungfernstieg_berg_mark.o3d",
        "Sceneryobjects/HamburgLinie109/model/2_K_jungfernstieg_berg_mark.o3d",
    ),
    (
        "Sceneryobjects/HafenCityHamburg/model/ggw_1.o3d",
        "Sceneryobjects/HamburgLinie109/model/ggw_1.o3d",
    ),
    (
        "Vehicles/HH20_C2_parked/Model/C2_VW_aussensp_aussen.o3d",
        "Vehicles/HC_C2_parked/Model/C2_VW_aussensp_aussen.o3d",
    ),
    (
        "Sceneryobjects/HamburgLi20/model/3_K_jahn_uebersee_west_mark.o3d",
        "Sceneryobjects/HafenCityHamburg/model/3_K_jahn_uebersee_west_mark.o3d",
    ),
];

fn load(root: &Path, rel: &str) -> Option<O3d> {
    parse(&std::fs::read(root.join(rel)).ok()?).ok()
}

#[test]
fn real_twin_pairs_decode_to_the_calibrated_exactness() {
    let Some(root) = omsi_root() else {
        eprintln!("no OMSI installation: real-pair test skipped");
        return;
    };
    let p = params();
    let (mut total, mut right, mut five_total, mut five_right) = (0, 0, 0, 0);
    let (mut exact_total, mut exact_right) = (0, 0);
    for (plain_rel, keyed_rel) in PAIRS {
        let (Some(plain), Some(keyed)) = (load(&root, plain_rel), load(&root, keyed_rel)) else {
            continue;
        };
        if plain.vertices.len() != keyed.vertices.len() {
            continue;
        }
        // The UV step with the true positions isolates the decoder from the position solver.
        let mut t = keyed.clone();
        for (v, truth) in t.vertices.iter_mut().zip(&plain.vertices) {
            v.position = truth.position;
        }
        let out = unjitter_uv_staged(&mut t, &[], &p);
        assert_eq!(
            out.quality.len(),
            t.vertices.len(),
            "{keyed_rel}: a vertex without quality"
        );
        let n = t.vertices.len();
        let r = exact(&t, &plain);
        total += n;
        right += r;
        if n % 5 == 0 {
            five_total += n;
            five_right += r;
        }
        for (i, q) in out.quality.iter().enumerate() {
            if *q == UvQuality::Exact {
                exact_total += 1;
                exact_right += usize::from(
                    (t.vertices[i].uv - plain.vertices[i].uv)
                        .abs()
                        .max_element()
                        < 1e-4,
                );
            }
        }
    }
    if total == 0 {
        eprintln!("none of the listed pairs is installed: skipped");
        return;
    }
    let rate = right as f64 / total as f64;
    let five_rate = five_right as f64 / five_total.max(1) as f64;
    eprintln!("exact {rate:.3} ({total} vertices), 5|n {five_rate:.3} ({five_total}), Exact-tier precision {:.4} ({exact_total})", exact_right as f64 / exact_total.max(1) as f64);
    assert!(rate >= 0.64, "overall exactness {rate:.3} (the pre-fix decoder reaches 0.58 on this sample, the fixed one 0.68)");
    assert!(
        five_rate >= 0.40,
        "5|n exactness {five_rate:.3} (pre-fix 0.30 on this sample, fixed 0.47)"
    );
    assert!(
        exact_right as f64 / exact_total.max(1) as f64 >= 0.97,
        "Exact tier is wrong too often: {exact_right}/{exact_total}"
    );
}

#[test]
fn a_keyed_mesh_without_twin_reports_unreliable_vertices() {
    // The GN92 steering wheel: keyed, no twin anywhere; the old decoder reported "0 % unresolved".
    let Some(root) = omsi_root() else {
        eprintln!("no OMSI installation: skipped");
        return;
    };
    let Some(mut wheel) = load(&root, "Vehicles/MAN_NL_NG/Model/GN92/GN92_Lenkrad.o3d") else {
        eprintln!("GN92 not installed: skipped");
        return;
    };
    let report = opts_core::formats::o3d::descramble::decode_confident(&mut wheel);
    assert!(report.accepted);
    assert_eq!(report.quality.len(), wheel.vertices.len());
    let unreliable = report
        .quality
        .iter()
        .filter(|q| **q == UvQuality::Unreliable)
        .count();
    assert!(
        unreliable > 0,
        "no vertex flagged: the silent failure is back"
    );
    assert_eq!(
        report.uv.unresolved, 0,
        "the old counter stays 0; the quality carries the truth"
    );
}
