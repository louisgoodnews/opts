//! Scenario driver: applies each scenario's light settings, waits for it to settle, then reads
//! the screenshot regions and records one result line.

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot, ScreenshotCaptured};
use opts_bevy::light_quality::{ladder_preset, LightMode, LightQualityInput};
use opts_bevy::mirrors::MirrorStats;
use opts_bevy::sky_cube::SkyCubeState;

use super::{Probe, GLASS_AT, MIRROR_AT, SETTLE_S};

/// Applies the current scenario's light-quality settings and starts its settle timer.
fn start_scenario(commands: &mut Commands, probe: &mut Probe, now: f32) {
    let idx = probe.idx;
    let (glass, mirrors) = (probe.scenarios[idx].glass, probe.scenarios[idx].mirrors);
    let mut q = ladder_preset(0);
    q.glass_reflections = glass;
    q.mirror_count = mirrors;
    q.mirror_every = 1;
    commands.insert_resource(LightQualityInput {
        mode: LightMode::Custom,
        custom: q,
    });
    probe.started_s = now;
    probe.shot_taken = false;
}

pub(super) fn drive(
    mut commands: Commands,
    time: Res<Time>,
    mut probe: ResMut<Probe>,
    stats: Res<MirrorStats>,
    cube: Res<SkyCubeState>,
    camera: Query<(&Camera, &GlobalTransform), With<opts_bevy::MainCamera>>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    if probe.idx >= probe.scenarios.len() {
        for l in &probe.lines {
            println!("{l}");
        }
        exit.write(AppExit::Success);
        return;
    }
    let idx = probe.idx;
    if probe.started_s < 0.0 {
        start_scenario(&mut commands, &mut probe, now);
        return;
    }
    if now - probe.started_s >= SETTLE_S && !probe.shot_taken {
        probe.shot_taken = true;
        let label = probe.scenarios[idx].label;
        let project = |p: Vec3| {
            camera
                .single()
                .ok()
                .and_then(|(c, gt)| c.world_to_viewport(gt, p).ok())
        };
        let (glass_px, mirror_px) = (project(GLASS_AT), project(MIRROR_AT));
        let (cameras, selected, rendering) = (stats.cameras, stats.selected, stats.rendering);
        let (builds, intensity) = (cube.builds, cube.intensity);
        let mut shot = commands.spawn(Screenshot::primary_window());
        if let Some(dir) = probe.shots.clone() {
            let _ = std::fs::create_dir_all(&dir);
            shot.observe(save_to_disk(dir.join(format!("{label}.png"))));
        }
        shot.observe(
            move |cap: On<ScreenshotCaptured>, mut probe: ResMut<Probe>| {
                let img = cap.image.clone().try_into_dynamic().expect("image").to_rgba8();
                let region = |centre: Option<Vec2>| -> String {
                    let Some(c) = centre else {
                        return "off-screen".into();
                    };
                    let (mut r, mut g, mut b, mut n) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
                    for y in (c.y as i32 - 12)..(c.y as i32 + 12) {
                        for x in (c.x as i32 - 20)..(c.x as i32 + 20) {
                            if let Some(p) = img.get_pixel_checked(x.max(0) as u32, y.max(0) as u32)
                            {
                                r += f32::from(p[0]);
                                g += f32::from(p[1]);
                                b += f32::from(p[2]);
                                n += 1.0;
                            }
                        }
                    }
                    format!("rgb {:.0} {:.0} {:.0}", r / n, g / n, b / n)
                };
                probe.lines.push(format!(
                    "{label}: glass {} | mirror {} | ground {} | sky {} | cameras {cameras} selected {selected} rendering {rendering} | cube builds {builds} intensity {intensity:.0}",
                    region(glass_px),
                    region(mirror_px),
                    region(Some(Vec2::new(120.0, 235.0))),
                    region(Some(Vec2::new(120.0, 30.0))),
                ));
                probe.idx += 1;
                probe.started_s = -1.0;
            },
        );
    }
}
