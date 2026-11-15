//! Frame-time statistics for `OPTS_REFLECT_STATS`: samples after a warm-up, then one summary
//! line written to the named file before the app exits.

use bevy::prelude::*;

#[derive(Resource, Default)]
pub(super) struct FrameStats {
    samples: Vec<f32>,
    started: Option<f64>,
}

fn rss_mib() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        })
        .map_or(0.0, |kb| kb / 1024.0)
}

pub(super) fn collect_stats(
    time: Res<Time<Real>>,
    mut stats: ResMut<FrameStats>,
    diagnostics: Option<Res<bevy::diagnostic::DiagnosticsStore>>,
    mut exit: MessageWriter<AppExit>,
    cameras: Query<(), With<crate::MainCamera>>,
) {
    let Ok(path) = std::env::var("OPTS_REFLECT_STATS") else {
        return;
    };
    // Statistics start when the 3D scene is on screen (a camera exists and frames are short).
    if cameras.is_empty() {
        return;
    }
    let now = time.elapsed_secs_f64();
    let start = *stats.started.get_or_insert(now);
    let warmup: f64 = std::env::var("OPTS_REFLECT_WARMUP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8.0);
    let span: f64 = std::env::var("OPTS_REFLECT_STATS_AFTER")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20.0);
    if now - start > warmup {
        stats.samples.push(time.delta_secs() * 1000.0);
    }
    if now - start > warmup + span {
        let mut v = stats.samples.clone();
        v.sort_by(|a, b| a.total_cmp(b));
        let n = v.len().max(1);
        let mean = v.iter().sum::<f32>() / n as f32;
        let pct = |p: f32| {
            v.get(((n as f32 - 1.0) * p) as usize)
                .copied()
                .unwrap_or(0.0)
        };
        let mut out = format!(
            "frames={} mean_ms={mean:.3} p50_ms={:.3} p95_ms={:.3} p99_ms={:.3} fps={:.1} rss_mib={:.0}\n",
            v.len(),
            pct(0.5),
            pct(0.95),
            pct(0.99),
            1000.0 / mean.max(1e-3),
            rss_mib()
        );
        if let Some(d) = diagnostics {
            for diag in d.iter() {
                let p = diag.path().as_str();
                if p.starts_with("render/") || p.contains("elapsed_gpu") {
                    if let Some(avg) = diag.average() {
                        out.push_str(&format!("diag {p} avg={avg:.4}\n"));
                    }
                }
            }
        }
        let _ = std::fs::write(&path, out);
        exit.write(AppExit::Success);
    }
}
