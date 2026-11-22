//! Unit tests of `bloom`.

use super::*;

fn app(quality: LightQuality) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(BloomPlugin);
    *app.world_mut().resource_mut::<LightQuality>() = quality;
    app
}

fn on(level: BloomLevel) -> LightQuality {
    LightQuality {
        bloom: level,
        ..LightQuality::default()
    }
}

fn camera(app: &mut App, hdr: bool) -> Entity {
    let mut e = app.world_mut().spawn(MainCamera);
    if hdr {
        e.insert(Hdr);
    }
    e.id()
}

#[test]
fn bloom_is_absent_when_off_and_restrained_when_on() {
    assert_eq!(bloom_for(&LightQuality::default()).map(|_| ()), None);
    let b = bloom_for(&on(BloomLevel::Natural)).expect("on");
    assert!(b.intensity > 0.0 && b.intensity < Bloom::NATURAL.intensity);
    assert_eq!(b.prefilter.threshold, BLOOM_THRESHOLD);
    assert_eq!(b.composite_mode, Bloom::NATURAL.composite_mode);
}

#[test]
fn the_strength_levels_rise_and_off_is_zero() {
    let steps: Vec<f32> = BloomLevel::ALL.iter().map(|l| intensity_of(*l)).collect();
    assert_eq!(steps[0], 0.0);
    assert!(steps.windows(2).all(|w| w[0] < w[1]), "{steps:?}");
}

#[test]
fn the_sync_adds_updates_and_removes_bloom_on_the_main_camera() {
    let mut app = app(on(BloomLevel::Natural));
    let cam = camera(&mut app, true);
    app.update();
    assert!(app.world().get::<Bloom>(cam).is_some());

    app.world_mut().resource_mut::<LightQuality>().bloom = BloomLevel::High;
    app.update();
    let i = app.world().get::<Bloom>(cam).expect("kept").intensity;
    assert_eq!(i, intensity_of(BloomLevel::High));

    app.world_mut().resource_mut::<LightQuality>().bloom = BloomLevel::Off;
    app.update();
    assert!(app.world().get::<Bloom>(cam).is_none());
}

#[test]
fn bloom_needs_hdr_and_does_not_switch_it_on() {
    let mut app = app(on(BloomLevel::Natural));
    let cam = camera(&mut app, false);
    app.update();
    assert!(app.world().get::<Bloom>(cam).is_none());
    assert!(app.world().get::<Hdr>(cam).is_none());
}

#[test]
fn bloom_is_removed_when_the_camera_loses_hdr() {
    let mut app = app(on(BloomLevel::Natural));
    let cam = camera(&mut app, true);
    app.update();
    assert!(app.world().get::<Bloom>(cam).is_some());
    app.world_mut().entity_mut(cam).remove::<Hdr>();
    app.update();
    assert!(app.world().get::<Bloom>(cam).is_none());
}

#[test]
fn other_cameras_never_get_bloom() {
    let mut app = app(on(BloomLevel::Natural));
    let other = app.world_mut().spawn(Hdr).id();
    app.update();
    assert!(app.world().get::<Bloom>(other).is_none());
}

#[test]
fn the_old_boolean_texts_still_mean_off_and_natural() {
    assert_eq!(BloomLevel::parse("false"), Some(BloomLevel::Off));
    assert_eq!(BloomLevel::parse("true"), Some(BloomLevel::Natural));
    for level in BloomLevel::ALL {
        assert_eq!(BloomLevel::parse(level.name()), Some(level));
    }
    assert_eq!(BloomLevel::parse("bright"), None);
}
