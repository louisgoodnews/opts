//! The per-frame drive of the plan.

use super::apply::apply;
use super::cone_material::ConeMaterial;
use super::*;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn drive(
    mut commands: Commands,
    time: Res<Time<Real>>,
    diag: Res<DiagnosticsStore>,
    mut bench: ResMut<Bench>,
    assets: Res<Assets3>,
    mut cones: ResMut<Assets<ConeMaterial>>,
    mut shadow_map: ResMut<DirectionalLightShadowMap>,
    cam: Query<Entity, With<Cam>>,
    mut images: ResMut<Assets<Image>>,
    mut every: ResMut<MirrorEvery>,
    stale_q: Query<Entity, Or<(With<Cone>, With<Fog>, With<Mirror>)>>,
    mut sun: Query<(Entity, &mut Transform, &mut Visibility), (With<Sun>, Without<Cam>)>,
    mut lamps: Query<(&NightLamp, &mut Visibility), (Without<Sun>, Without<HeadSpot>)>,
    mut spots: Query<
        (Entity, &HeadSpot, &mut SpotLight, &mut Visibility),
        (Without<Sun>, Without<NightLamp>),
    >,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut exit: MessageWriter<AppExit>,
) {
    if bench.idx >= bench.plan.len() {
        exit.write(AppExit::Success);
        return;
    }
    let cfg = bench.plan[bench.idx].clone();
    let warm = cfg.u("warmup", 120);
    let frames = cfg.u("frames", 600);
    let shot_at = warm + 4;
    let measure_from = if cfg.u("shot", 0) == 1 {
        warm + 24
    } else {
        warm
    };

    if bench.frame == 0 {
        apply(
            &cfg,
            &mut commands,
            &assets,
            &mut cones,
            &mut shadow_map,
            cam.single().ok(),
            &mut images,
            &mut every,
            &stale_q,
            &mut sun,
            &mut lamps,
            &mut spots,
            &mut ambient,
        );
        bench.ms.clear();
        bench.gpu.clear();
        bench.item_start = std::time::Instant::now();
        bench.shot_done = false;
        bench.frame = 1;
        return;
    }
    bench.frame += 1;
    let f = bench.frame;
    let warm_s = cfg.f("warm_s", if bench.idx == 0 { 4.0 } else { 1.2 });
    let warmed = bench.item_start.elapsed().as_secs_f32() >= warm_s;
    if f >= shot_at && warmed && !bench.shot_done && cfg.u("shot", 0) == 1 {
        bench.shot_done = true;
        if let Some(dir) = bench.shots.clone() {
            let _ = std::fs::create_dir_all(&dir);
            let shot = match &assets.target {
                Some(h) => Screenshot::image(h.clone()),
                None => Screenshot::primary_window(),
            };
            commands
                .spawn(shot)
                .observe(save_to_disk(dir.join(format!("{}.png", cfg.label))));
        }
    }
    let shot_wait = cfg.u("shot", 0) == 1 && (!bench.shot_done || f < shot_at + 20);
    if f > measure_from && warmed && !shot_wait {
        bench.ms.push(time.delta_secs() * 1000.0);
        for d in diag.iter() {
            let p = d.path().as_str();
            if p.ends_with("elapsed_gpu") {
                if let Some(v) = d.value() {
                    bench.gpu.entry(p.to_string()).or_default().push(v);
                }
            }
        }
    }
    let over_time = bench.started.elapsed().as_secs_f32() > bench.max_seconds;
    if bench.ms.len() as u32 >= frames || over_time {
        let mut ms = bench.ms.clone();
        ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = ms.len().max(1) as f32;
        let mean = ms.iter().sum::<f32>() / n;
        let mut gpu = serde_json::Map::new();
        for (k, v) in &bench.gpu {
            let mut s = v.clone();
            s.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let med = s[s.len() / 2];
            gpu.insert(
                k.clone(),
                serde_json::json!((med * 1000.0).round() / 1000.0),
            );
        }
        let gpu_sum: f64 = bench
            .gpu
            .iter()
            .filter(|(k, _)| {
                k.contains("main_opaque_pass_3d")
                    || k.contains("main_transparent_pass_3d")
                    || k.contains("main_transmissive")
                    || k.contains("shadow")
                    || k.contains("volumetric")
                    || k.contains("prepass")
                    || k.contains("post_processing")
                    || k.contains("tonemapping")
                    || k.contains("msaa")
            })
            .map(|(_, v)| {
                let mut s = v.clone();
                s.sort_by(|a, b| a.partial_cmp(b).unwrap());
                s[s.len() / 2]
            })
            .sum();
        let rec = serde_json::json!({
            "label": cfg.label,
            "cfg": cfg.kv,
            "frames": ms.len(),
            "median_ms": pct(&ms, 0.5),
            "p95_ms": pct(&ms, 0.95),
            "p99_ms": pct(&ms, 0.99),
            "max_ms": ms.last().copied().unwrap_or(0.0),
            "min_fps": 1000.0 / ms.last().copied().unwrap_or(1.0).max(0.001),
            "mean_ms": mean,
            "gpu_median_ms": gpu,
            "gpu_sum_known_ms": gpu_sum,
            "loadavg_start": bench.load,
            "truncated": over_time,
        });
        let line = rec.to_string();
        println!("RESULT {line}");
        if let Some(out) = &bench.out {
            if let Ok(mut fh) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(out)
            {
                let _ = writeln!(fh, "{line}");
            }
        }
        bench.idx += 1;
        bench.frame = 0;
        if over_time {
            bench.idx = bench.plan.len();
        }
    }
}
