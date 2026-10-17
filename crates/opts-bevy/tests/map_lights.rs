//! Headless test of the `[maplight]` pool: the nearest lamps get the pool's point lights at night,
//! none by day, and the lights fade in.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use opts_bevy::scenery_runtime::{
    spawn_map_lights, MapLight, MapLightPool, MapLightSettings, SceneryRuntimePlugin,
};
use opts_bevy::SceneryClock;
use opts_core::formats::scenery::SceneryObject;

/// A lamp object with one `[maplight]` 9 m up, radius 4.5 m.
const LAMP: &str = "[maplight]\n0\n0\n9\n1\n0.63\n0.31\n4.5\n";

fn app(hour: f64, lamps: usize) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            watch_for_changes_override: Some(false),
            ..Default::default()
        },
        bevy::transform::TransformPlugin,
    ))
    .init_resource::<Assets<Mesh>>()
    .init_resource::<Assets<StandardMaterial>>()
    .add_plugins(SceneryRuntimePlugin)
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        50,
    )))
    .insert_resource(SceneryClock::at(2026, 6, 29, hour * 3600.0))
    .insert_resource(MapLightSettings {
        pool_size: 4,
        ..Default::default()
    });
    let sco = SceneryObject::from_text(LAMP).expect("sco");
    app.add_systems(Startup, move |mut commands: Commands| {
        // Lamps every 30 m along x; the camera at the first one.
        for i in 0..lamps {
            let root = commands
                .spawn((
                    Transform::from_xyz(i as f32 * 30.0, 0.0, 0.0),
                    Visibility::Inherited,
                ))
                .id();
            spawn_map_lights(&mut commands, root, &sco);
        }
        commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 2.0, 0.0)));
    });
    app
}

fn lit(app: &mut App) -> Vec<(f32, f32)> {
    let mut q = app.world_mut().query::<(&PointLight, &GlobalTransform)>();
    let mut out: Vec<(f32, f32)> = q
        .iter(app.world())
        .filter(|(p, _)| p.intensity > 0.0)
        .map(|(p, t)| (t.translation().x, p.intensity))
        .collect();
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

#[test]
fn at_night_the_nearest_lamps_get_the_pool_and_fade_in() {
    let mut app = app(23.0, 10);
    for _ in 0..3 {
        app.update();
    }
    let early = lit(&mut app);
    for _ in 0..20 {
        app.update();
    }
    let full = lit(&mut app);
    let xs: Vec<f32> = full.iter().map(|l| l.0).collect();
    assert_eq!(xs, vec![0.0, 30.0, 60.0, 90.0], "the four nearest lamps");
    let lumens = opts_bevy::scenery_runtime::map_light_lumens(4.5, 15.0);
    assert!(full.iter().all(|l| (l.1 - lumens).abs() < 1.0));
    assert!(early.iter().all(|l| l.1 < lumens), "fading in: {early:?}");
    // Stable: the same lamps keep the lights.
    let pool_before = app.world().resource::<MapLightPool>().assigned();
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        app.world().resource::<MapLightPool>().assigned(),
        pool_before
    );
    let lamps = app
        .world_mut()
        .query::<&MapLight>()
        .iter(app.world())
        .count();
    assert_eq!(lamps, 10);
}

#[test]
fn by_day_no_lamp_is_lit() {
    let mut app = app(12.0, 10);
    for _ in 0..20 {
        app.update();
    }
    assert!(lit(&mut app).is_empty());
}
