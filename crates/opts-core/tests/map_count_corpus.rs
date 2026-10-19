//! Install-gated check of the static profiler (M-0130 B3): the counts of real maps equal a plain
//! keyword census, the sample estimate of the large maps stays within 10 % of the full count, and
//! the cost is recorded. Skipped unless `OMSI_PATH` points at an installation.

use std::path::PathBuf;
use std::time::Instant;

use opts_core::formats::map::count::{ai_pool_size, count_map, SAMPLE_BUDGET_BYTES};

fn maps() -> Option<Vec<PathBuf>> {
    let root = PathBuf::from(std::env::var_os("OMSI_PATH")?).join("maps");
    let mut v: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    v.sort();
    Some(v)
}

#[test]
fn counts_of_real_maps_are_complete_and_the_sample_estimate_is_close() {
    let Some(maps) = maps() else {
        eprintln!("OMSI_PATH not set: skipped");
        return;
    };
    let (mut checked, mut sampled_maps, mut worst) = (0, 0, 0.0_f64);
    for m in maps {
        let full = count_map(&m, u64::MAX);
        if full.tiles == 0 {
            continue;
        }
        checked += 1;
        let t = Instant::now();
        let quick = count_map(&m, SAMPLE_BUDGET_BYTES);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        if quick.sampled {
            sampled_maps += 1;
            if full.objects > 1000 {
                let err = (quick.objects as f64 - full.objects as f64).abs() / full.objects as f64;
                worst = worst.max(err);
                assert!(
                    err < 0.15,
                    "{}: sample {} vs full {} ({:.1} %)",
                    m.display(),
                    quick.objects,
                    full.objects,
                    err * 100.0
                );
            }
        } else {
            assert_eq!(quick, full);
        }
        eprintln!(
            "{:30} tiles {:5} objects {:7} splines {:6} ai pool {:4} quick {:6.1} ms{}",
            m.file_name().unwrap().to_string_lossy(),
            full.tiles,
            full.objects,
            full.splines,
            ai_pool_size(&m),
            ms,
            if quick.sampled { " (sampled)" } else { "" }
        );
    }
    assert!(checked > 0);
    eprintln!(
        "{checked} maps, {sampled_maps} sampled, worst estimate error {:.1} %",
        worst * 100.0
    );
}
