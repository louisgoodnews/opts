//! Unit tests for the hardware benchmark: quick runs, progress, storage step cleanup and cancel.
use super::*;

fn quick() -> BenchOptions {
    BenchOptions {
        cpu_time: Duration::from_millis(20),
        memory_mib: 4,
        storage_dir: None,
        storage_mib: 2,
    }
}

#[test]
fn runs_and_reports_progress() {
    let mut p = HardwareProfile::default();
    let mut seen = Vec::new();
    run(&mut p, &quick(), &CancelToken::new(), &mut |s, i, n| {
        seen.push((s, i, n))
    });
    assert_eq!(seen.len(), 4);
    assert_eq!(seen[3], (Step::Storage, 3, 4));
    assert!(p.bench.cpu_single_mops.get().is_some_and(|v| v > 0));
    assert!(p.bench.cpu_multi_mops.get().is_some());
    assert!(p.bench.memory_mib_s.get().is_some_and(|v| v > 0));
    assert_eq!(
        p.bench.storage_read_mib_s,
        Fact::Unknown(Reason::NotRequested)
    );
    assert_eq!(p.bench.measured_for_class, Fact::Known(p.class_hash()));
}

#[test]
fn storage_step_leaves_no_file_behind() {
    let dir = std::env::temp_dir().join(format!("opts-hw-bench-test-{}", std::process::id()));
    let mut p = HardwareProfile::default();
    let o = BenchOptions {
        storage_dir: Some(dir.clone()),
        ..quick()
    };
    run(&mut p, &o, &CancelToken::new(), &mut |_, _, _| {});
    assert!(p.bench.storage_read_mib_s.get().is_some());
    assert_eq!(fs::read_dir(&dir).map(Iterator::count).unwrap_or(0), 0);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cancel_stops_the_run() {
    let c = CancelToken::new();
    let mut p = HardwareProfile::default();
    let c2 = c.clone();
    run(&mut p, &quick(), &c, &mut |s, _, _| {
        if s == Step::CpuMulti {
            c2.cancel();
        }
    });
    assert!(p.bench.cpu_single_mops.get().is_some());
    assert_eq!(p.bench.cpu_multi_mops, Fact::Unknown(Reason::Cancelled));
    assert_eq!(p.bench.memory_mib_s, Fact::Unknown(Reason::Cancelled));
    assert!(p.bench.measured_for_class.known().is_none());
}
