//! Criterion benchmarks of the OMSI text parsers (M-0028), on synthetic input written for this
//! file: no OMSI asset is committed, and the numbers do not depend on an installation.
//!
//! Run with `cargo bench -p opts-benchmark --no-default-features --bench parsers`. The sizes are
//! chosen to look like real files (a dense city tile holds thousands of objects; a bus file a few
//! kilobytes), so the figures show where the time of a load goes, not a best case.
//!
//! The generators are checked once per run: a generator that stops producing what the parser reads
//! would otherwise give a fast, meaningless number.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use opts_core::formats::map::MapTile;
use opts_core::formats::scenery::SceneryObject;
use opts_core::formats::vehicles::BusManifest;

/// A version-14 tile with `objects` placed `[object]`s (two per hundred have text variables).
fn tile_text(objects: usize) -> String {
    let mut s = String::from("[version]\n14\n\n");
    for i in 0..objects {
        s.push_str(&format!(
            "[object]\n0\nSceneryobjects\\Generic\\item_{}.sco\n{}\n{}\n{}\n0\n{}\n0\n0\n1\n\n\nObject Nr. {i}\n\n",
            i % 40,
            10_000 + i,
            (i * 7) % 300,
            (i * 13) % 300,
            (i * 29) % 360,
        ));
    }
    s
}

/// A scenery object with `meshes` meshes, each with a material and a visibility rule.
fn scenery_text(meshes: usize) -> String {
    let mut s = String::from("[groups]\n2\nAachen\nAmpeln\n\n[friendlyname]\nBench object\n\n");
    for i in 0..meshes {
        s.push_str(&format!(
            "[mesh]\nmesh_{i}.o3d\n\n[matl]\ntexture_{i}.bmp\n0\n\n[visible]\nvariable_{i}\n1\n\n"
        ));
    }
    s.push_str("[boundingbox]\n0.3\n0.3\n1.2\n0.0\n0.0\n0.6\n");
    s
}

/// A bus manifest like the installed ones (a few kilobytes) with `axles` axle blocks.
fn bus_text(axles: usize) -> String {
    let mut s = String::from(
        "[friendlyname]\nMAN\nNL202\nBVG Standard\n\n[description]\nStandard Berliner Linienbus\nBaujahr 1992\n[end]\n\n\
[number]\nRegs_NL202.org\n\n[sound]\nsound\\sound_NL202.cfg\n\n[paths]\nmodel\\paths_NL202.cfg\n\n\
[passengercabin]\nmodel\\passengercabin_NL202.cfg\n\n[model]\nmodel\\model_NL202.cfg\n\n\
[varnamelist]\n2\nscript\\engine_varlist.txt\nscript\\door_varlist.txt\n\n[script]\n1\nscript\\engine.osc\n\n\
[constfile]\n1\nscript\\engine_constfile.txt\n\n[mass]\n11.5\n\n",
    );
    for i in 0..axles {
        s.push_str(&format!(
            "[newachse]\nachse_long\n{:.3}\nachse_maxwidth\n2.25\nachse_antrieb\n{}\n\n",
            1.5 + i as f32 * 2.7,
            i % 2
        ));
    }
    s
}

fn parsers(c: &mut Criterion) {
    // Generators are checked once: they must give what the parser reads.
    assert_eq!(
        MapTile::from_text(&tile_text(50)).unwrap().objects.len(),
        50
    );
    assert_eq!(
        SceneryObject::from_text(&scenery_text(10))
            .unwrap()
            .meshes
            .len(),
        10
    );
    assert!(BusManifest::parse(&bus_text(3)).is_ok());

    let mut group = c.benchmark_group("tile");
    for objects in [500, 5_000] {
        let text = tile_text(objects);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_function(format!("{objects}_objects"), |b| {
            b.iter_batched(
                || text.as_str(),
                |t| black_box(MapTile::from_text(black_box(t)).unwrap()),
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();

    let mut group = c.benchmark_group("scenery_object");
    for meshes in [5, 60] {
        let text = scenery_text(meshes);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_function(format!("{meshes}_meshes"), |b| {
            b.iter(|| black_box(SceneryObject::from_text(black_box(&text)).unwrap()));
        });
    }
    group.finish();

    let mut group = c.benchmark_group("bus_manifest");
    for axles in [3, 40] {
        let text = bus_text(axles);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_function(format!("{axles}_axles"), |b| {
            b.iter(|| black_box(BusManifest::parse(black_box(&text)).unwrap()));
        });
    }
    group.finish();
}

criterion_group!(benches, parsers);
criterion_main!(benches);
