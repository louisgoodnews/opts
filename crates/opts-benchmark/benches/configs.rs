//! Criterion benchmarks of two more OMSI text parsers (M-0028), on synthetic input written for
//! this file: a map's `global.cfg` and a vehicle's `model.cfg`. No OMSI asset is needed.
//!
//! Run with `cargo bench -p opts-benchmark --no-default-features --bench configs`. The sizes follow
//! real files: a city map lists thousands of tiles, a bus model a few hundred meshes. Each
//! generator is checked once per run, so a generator that stops giving what the parser reads
//! cannot produce a fast, meaningless number.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use opts_core::formats::map::MapManifest;
use opts_core::formats::vehicles::model::ModelConfig;

/// A map's `global.cfg` with `tiles` tile entries on a square grid.
fn global_text(tiles: usize) -> String {
    let mut s = String::from(
        "[name]\nBenchmark\n\n[friendlyname]\nBenchmark\n\n[description]\nA synthetic map.\n[end]\n\n\
[version]\n14\n\n[NextIDCode]\n322886\n\n[moneysystem]\nMoney\\DM\\DM.cfg\n\n\
[ticketpack]\nTicketpacks\\Berlin_1\\Berlin_1.otp\n\n[standarddepot]\nBenchmark\n\n",
    );
    let side = (tiles as f64).sqrt().ceil() as i64;
    for i in 0..tiles as i64 {
        let (x, y) = (i % side, -(i / side));
        s.push_str(&format!("[map]\n{x}\n{y}\ntile_{x}_{y}.map\n\n"));
    }
    s
}

/// A `model.cfg` with `meshes` meshes, each with a material, a visibility rule and an animation.
fn model_text(meshes: usize) -> String {
    let mut s = String::from("[tex_detail_factor]\n0.5\n\n[LOD]\n0.1\n\n");
    for i in 0..meshes {
        s.push_str(&format!(
            "[mesh]\nGeneric\\Part_{i}.o3d\n\n[viewpoint]\n1\n\n[mesh_ident]\npart_{i}\n\n\
[matl]\nTexture_{}.bmp\n0\n\n[matl_alpha]\n2\n\n[visible]\nvisible_{}\n1.0\n\n\
[newanim]\norigin_trans\n-0.65\n4.85\n1.15\norigin_rot_x\n-15.0\nanim_rot\nrotation_{}\n90.0\nmaxspeed\n1.5\n\n",
            i % 12,
            i % 30,
            i % 40
        ));
    }
    s
}

fn configs(c: &mut Criterion) {
    // Generators are checked once: they must give what the parser reads.
    assert_eq!(
        MapManifest::from_text(&global_text(50))
            .unwrap()
            .tiles
            .len(),
        50
    );
    let sample = ModelConfig::parse(&model_text(20)).expect("the generated model parses");
    assert_eq!(
        sample.lods.iter().map(|l| l.meshes.len()).sum::<usize>(),
        20
    );

    let mut group = c.benchmark_group("global_cfg");
    for tiles in [100, 2_000] {
        let text = global_text(tiles);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_function(format!("{tiles}_tiles"), |b| {
            b.iter(|| black_box(MapManifest::from_text(black_box(&text)).unwrap()));
        });
    }
    group.finish();

    let mut group = c.benchmark_group("model_cfg");
    for meshes in [30, 300] {
        let text = model_text(meshes);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_function(format!("{meshes}_meshes"), |b| {
            b.iter(|| black_box(ModelConfig::parse(black_box(&text)).unwrap()));
        });
    }
    group.finish();
}

criterion_group!(benches, configs);
criterion_main!(benches);
