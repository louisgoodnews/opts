//! Criterion benchmark of the spline profile (`.sli`) parser (M-0028), on synthetic input written
//! for this file in the layout of the format page's minimal example (`formats/text/sli.md`,
//! shortened from `RQ_10,5_2spur_7,5m_Sidewalk.sli`). No OMSI asset is needed.
//!
//! Run with `cargo bench -p opts-benchmark --no-default-features --bench splines`. The corpus of
//! 12,382 files averages about 4 `[profile]`, 10 `[profilepnt]`, 2 `[texture]`, 2
//! `[heightprofile]` and 1 `[path]` per file (keyword counts in the format page), so the small
//! case is a typical road; the large one is ten times that (the largest file was not measured).
//! The generator is checked once per run, so it cannot produce a fast, meaningless number.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use opts_core::formats::scenery::sli::SplineProfile;

/// Points per `[profile]`: a kerb-to-kerb strip of the minimal example has two, a kerb three.
const POINTS_PER_PROFILE: usize = 3;

/// A `.sli` with `profiles` graphical profiles of three points each, and one texture and one
/// height profile per two profiles and one path per four, in the minimal example's layout.
fn sli_text(profiles: usize) -> String {
    let mut s = String::new();
    for i in 0..profiles.div_ceil(2) {
        let x0 = -6.0 + i as f32;
        s.push_str(&format!(
            "[heightprofile]\n{x0:.3}\n{:.3}\n0.100\n0.100\n\n",
            x0 + 1.0
        ));
    }
    for i in 0..profiles.div_ceil(2) {
        s.push_str(&format!("[texture]\nstr_texture_{i}.bmp\n\n"));
    }
    for p in 0..profiles {
        s.push_str(&format!("[profile]\n{}\n\n", p / 2));
        for k in 0..POINTS_PER_PROFILE {
            let x = -6.0 + p as f32 * 0.5 + k as f32 * 0.25;
            s.push_str(&format!(
                "[profilepnt]\n{x:.3}\n{:.3}\n{:.3}\n0.200\n\n",
                0.1 + 0.05 * k as f32,
                k as f32 * 0.5
            ));
        }
    }
    for i in 0..profiles.div_ceil(4) {
        s.push_str(&format!(
            "[path]\n0\n{:.3}\n0.100\n3.535\n{}\n\n",
            -1.875 + 3.75 * i as f32,
            i % 2
        ));
    }
    s
}

fn splines(c: &mut Criterion) {
    // The generator is checked once: it must give what the parser reads.
    let sample = SplineProfile::from_text(&sli_text(6)).expect("the generated profile parses");
    assert_eq!(sample.graphical_profiles.len(), 6);
    assert!(sample
        .graphical_profiles
        .iter()
        .all(|p| p.points.len() == POINTS_PER_PROFILE));
    assert_eq!(
        (
            sample.textures.len(),
            sample.height_profiles.len(),
            sample.paths.len()
        ),
        (3, 3, 2)
    );

    let mut group = c.benchmark_group("sli");
    for profiles in [4, 40] {
        let text = sli_text(profiles);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_function(format!("{profiles}_profiles"), |b| {
            b.iter(|| black_box(SplineProfile::from_text(black_box(&text)).unwrap()));
        });
    }
    group.finish();
}

criterion_group!(benches, splines);
criterion_main!(benches);
