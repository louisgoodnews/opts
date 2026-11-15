//! The probe capture: the luminance report of a frame and the driving of a scenario.

use super::*;

/// Mean, peak and centre luminance of a captured frame, as one report line.
fn frame_line(label: &str, img: &image::RgbaImage) -> String {
    let (w, h) = img.dimensions();
    let luma = |p: &image::Rgba<u8>| {
        (0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2])) / 255.0
    };
    let (mut sum, mut peak, mut peak_ch) = (0.0f32, 0.0f32, 0.0f32);
    for p in img.pixels() {
        let l = luma(p);
        sum += l;
        peak = peak.max(l);
        peak_ch = peak_ch.max(f32::from(p[0].max(p[1]).max(p[2])) / 255.0);
    }
    let mut centre = 0.0f32;
    let mut n = 0.0f32;
    for y in (h / 2 - 20)..(h / 2 + 20) {
        for x in (w / 2 - 20)..(w / 2 + 20) {
            centre += luma(img.get_pixel(x, y));
            n += 1.0;
        }
    }
    format!(
        "{label}: mean {:.4} peak_luma {peak:.3} peak_channel {peak_ch:.3} centre_mean {:.4}",
        sum / (w * h) as f32,
        centre / n
    )
}

pub(super) fn drive(
    mut commands: Commands,
    time: Res<Time>,
    mut probe: ResMut<Probe>,
    lamps: Query<Entity, With<Lamp>>,
    mut camera: Query<&mut Transform, With<opts_bevy::MainCamera>>,
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
        setup_scenario(&mut commands, &mut probe, &lamps, &mut camera);
        probe.started_s = now;
        probe.shot_taken = false;
        return;
    }
    if now - probe.started_s >= SETTLE_S && !probe.shot_taken {
        probe.shot_taken = true;
        let label = probe.scenarios[idx].label;
        let mut shot = commands.spawn(Screenshot::primary_window());
        if let Some(dir) = probe.shots.clone() {
            let _ = std::fs::create_dir_all(&dir);
            shot.observe(save_to_disk(dir.join(format!("{label}.png"))));
        }
        shot.observe(
            move |cap: On<ScreenshotCaptured>, mut probe: ResMut<Probe>| {
                let img = cap
                    .image
                    .clone()
                    .try_into_dynamic()
                    .expect("image")
                    .to_rgba8();
                probe.lines.push(frame_line(label, &img));
                probe.idx += 1;
                probe.started_s = -1.0;
            },
        );
    }
}
