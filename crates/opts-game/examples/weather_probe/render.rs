use super::*;
use super::scene::step;

/// The camera heights (`OPTS_PROBE_HEIGHTS`, default 2 m) and the pitch (`OPTS_PROBE_PITCH`).
pub(super) fn probe_view() -> (Vec<f32>, Option<f32>) {
    let heights: Vec<f32> = std::env::var("OPTS_PROBE_HEIGHTS")
        .ok()
        .map(|v| v.split(',').filter_map(|x| x.trim().parse().ok()).collect())
        .unwrap_or_default();
    let pitch: Option<f32> = std::env::var("OPTS_PROBE_PITCH")
        .ok()
        .and_then(|v| v.parse().ok());
    let heights = if heights.is_empty() {
        vec![2.0]
    } else {
        heights
    };
    (heights, pitch)
}

/// Renders one preset at one camera height, reads it back and writes the PNG.
pub(super) fn render_job(
    apps: &mut SubApps,
    shared: &Arc<Mutex<Vec<u8>>>,
    target: &Handle<Image>,
    job: (usize, &str, f32),
    pitch: Option<f32>,
    out: &str,
) {
    let (n, id, height) = job;
    {
        let mut q = apps
            .main
            .world_mut()
            .query_filtered::<&mut Transform, With<InGameCamera>>();
        let mut t = q.single_mut(apps.main.world_mut()).unwrap();
        let target = match pitch {
            Some(p) => Vec3::new(0.0, height + 60.0 * p.to_radians().tan(), -60.0),
            None => Vec3::new(0.0, height + 4.0, -60.0),
        };
        *t = Transform::from_xyz(0.0, height, 0.0).looking_at(target, Vec3::Y);
    }
    let hour = if id.to_lowercase().contains("daemmer") {
        19.0
    } else {
        12.0
    };
    apps.main.world_mut().write_message(EnvironmentChange {
        date: Some(CalendarDate::new(2026, 6, 29)),
        seconds_of_day: Some(hour * 3600.0),
        weather: Some(id.to_string()),
        ..default()
    });
    for _ in 0..40 {
        step(apps);
    }
    shared.lock().unwrap().clear();
    let sink = shared.clone();
    let e = apps
        .main
        .world_mut()
        .spawn(Readback::texture(target.clone()))
        .observe(move |ev: On<ReadbackComplete>| {
            if let Ok(mut v) = sink.lock() {
                *v = ev.data.clone();
            }
        })
        .id();
    for _ in 0..4 {
        step(apps);
    }
    apps.main.world_mut().despawn(e);
    let data = shared.lock().unwrap().clone();
    let name: String = id
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    if let Some(img) = image::RgbaImage::from_raw(W, H, data) {
        let path = format!("{out}/{n:02}_{name}_h{height:.0}.png");
        let _ = img.save(&path);
        println!("{id}: {path}");
    } else {
        println!("{id}: no readback");
    }
}
