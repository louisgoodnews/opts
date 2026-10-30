use super::*;

fn state(puddles: f32) -> SurfaceState {
    SurfaceState {
        puddles,
        ..Default::default()
    }
}

#[test]
fn the_target_follows_coverage_and_wetness() {
    assert_eq!(puddle_target(&state(0.0), 1.0), 0);
    assert_eq!(puddle_target(&state(0.45), 0.0), 0);
    assert_eq!(puddle_target(&state(0.45), 1.0), MAX_PUDDLES);
    assert_eq!(puddle_target(&state(0.225), 1.0), MAX_PUDDLES / 2);
    assert_eq!(puddle_target(&state(0.45), 0.5), MAX_PUDDLES / 2);
    // More than full coverage does not add more.
    assert_eq!(puddle_target(&state(0.9), 1.0), MAX_PUDDLES);
}

#[test]
fn a_puddle_is_opaque_in_the_middle_and_gone_at_the_corners() {
    assert!(blob_alpha(0.5, 0.5) > 0.99);
    for (u, v) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
        assert!(blob_alpha(u, v) < 0.01, "({u}, {v})");
    }
    // Irregular: the edge is not a circle.
    let radii: Vec<f32> = (0..16)
        .map(|k| {
            let a = k as f32 / 16.0 * TAU;
            (0..200)
                .map(|i| i as f32 / 200.0)
                .take_while(|r| blob_alpha(0.5 + 0.5 * r * a.cos(), 0.5 + 0.5 * r * a.sin()) > 0.5)
                .last()
                .unwrap_or(0.0)
        })
        .collect();
    let (lo, hi) = radii
        .iter()
        .fold((f32::MAX, f32::MIN), |(l, h), r| (l.min(*r), h.max(*r)));
    assert!(hi - lo > 0.1, "{radii:?}");
}

#[test]
fn rain_intensity_raises_the_wetness_and_with_it_the_puddles() {
    use opts_bevy::weather::surface_state_for;
    let c = opts_bevy::WeatherConditions {
        ground_wet: [0.0, 120.0, 60.0],
        ..Default::default()
    };
    let half = surface_state_for(&c, 0.5, 1.0);
    let full = surface_state_for(&c, 1.0, 1.0);
    let heavy = surface_state_for(&c, 2.0, 1.0);
    assert!(half.puddles < full.puddles && full.puddles < heavy.puddles);
    assert!(half.wet < full.wet && full.wet < heavy.wet);
    assert_eq!(surface_state_for(&c, 0.0, 1.0).puddles, 0.0);
}
