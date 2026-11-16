//! The measured workloads of the benchmark: CPU, memory and storage.

use super::*;

/// One thread of xorshift work until `until`; returns the number of operations.
pub(super) fn spin(until: Instant, cancel: &CancelToken) -> u64 {
    let (mut x, mut ops) = (0x9E37_79B9_7F4A_7C15_u64, 0_u64);
    while Instant::now() < until && !cancel.is_cancelled() {
        for _ in 0..100_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
        }
        ops += 100_000 * 3;
    }
    std::hint::black_box(x);
    ops
}

pub(super) fn mops(ops: u64, elapsed: Duration) -> u32 {
    let s = elapsed.as_secs_f64().max(1e-6);
    u32::try_from((ops as f64 / s / 1e6) as u64).unwrap_or(u32::MAX)
}

pub(super) fn cpu(threads: usize, time: Duration, cancel: &CancelToken) -> Fact<u32> {
    let start = Instant::now();
    let until = start + time;
    let ops: u64 = std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads.max(1))
            .map(|_| s.spawn(|| spin(until, cancel)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or(0)).sum()
    });
    if cancel.is_cancelled() {
        Fact::Unknown(Reason::Cancelled)
    } else {
        Fact::Known(mops(ops, start.elapsed()))
    }
}

pub(super) fn memory(mib: usize, cancel: &CancelToken) -> Fact<u32> {
    let len = mib.max(1) * 1024 * 1024;
    let src = vec![1_u8; len];
    let mut dst = vec![0_u8; len];
    let start = Instant::now();
    let mut copied = 0_usize;
    while start.elapsed() < Duration::from_millis(200) && !cancel.is_cancelled() {
        dst.copy_from_slice(&src);
        std::hint::black_box(&dst);
        copied += len;
    }
    if cancel.is_cancelled() {
        return Fact::Unknown(Reason::Cancelled);
    }
    let rate = copied as f64 / 1_048_576.0 / start.elapsed().as_secs_f64().max(1e-6);
    Fact::Known(u32::try_from(rate as u64).unwrap_or(u32::MAX))
}

/// Writes then reads back a scratch file (deleted afterwards). The read may be served by the OS
/// cache, so the figure is an upper bound of the drive; it still separates SSD from HDD classes
/// when the file exceeds the cache.
pub(super) fn storage(dir: &std::path::Path, mib: usize, cancel: &CancelToken) -> Fact<u32> {
    let file = dir.join(format!("opts-hw-bench-{}.tmp", std::process::id()));
    let result = (|| -> Result<Fact<u32>, std::io::Error> {
        fs::create_dir_all(dir)?;
        let chunk = vec![0xA5_u8; 1024 * 1024];
        let mut f = fs::File::create(&file)?;
        for _ in 0..mib.max(1) {
            if cancel.is_cancelled() {
                return Ok(Fact::Unknown(Reason::Cancelled));
            }
            f.write_all(&chunk)?;
        }
        f.sync_all()?;
        drop(f);
        let mut f = fs::File::open(&file)?;
        let mut buf = vec![0_u8; 1024 * 1024];
        let (start, mut total) = (Instant::now(), 0_usize);
        loop {
            if cancel.is_cancelled() {
                return Ok(Fact::Unknown(Reason::Cancelled));
            }
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            total += n;
        }
        let rate = total as f64 / 1_048_576.0 / start.elapsed().as_secs_f64().max(1e-6);
        Ok(Fact::Known(u32::try_from(rate as u64).unwrap_or(u32::MAX)))
    })();
    let _ = fs::remove_file(&file);
    result.unwrap_or(Fact::Unknown(Reason::Unreadable))
}
