//! Unit tests of `bloom`.

use super::*;

fn app(quality: LightQuality) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(BloomPlugin);
    *app.world_mut().resource_mut::<LightQuality>() = quality;
    app
}

fn on() -> LightQuality {
    LightQuality {
        bloom: true,
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
    let b = bloom_for(&on()).expect("on");
    assert!(b.intensity > 0.0 && b.intensity < Bloom::NATURAL.intensity);
    assert_eq!(b.prefilter.threshold, BLOOM_THRESHOLD);
    assert_eq!(b.composite_mode, Bloom::NATURAL.composite_mode);
}

#[test]
fn the_sync_adds_and_removes_bloom_on_the_main_camera() {
    let mut app = app(on());
    let cam = camera(&mut app, true);
    app.update();
    assert!(app.world().get::<Bloom>(cam).is_some());

    app.world_mut().resource_mut::<LightQuality>().bloom = false;
    app.update();
    assert!(app.world().get::<Bloom>(cam).is_none());
}

#[test]
fn bloom_needs_hdr_and_does_not_switch_it_on() {
    let mut app = app(on());
    let cam = camera(&mut app, false);
    app.update();
    assert!(app.world().get::<Bloom>(cam).is_none());
    assert!(app.world().get::<Hdr>(cam).is_none());
}

#[test]
fn other_cameras_never_get_bloom() {
    let mut app = app(on());
    let other = app.world_mut().spawn(Hdr).id();
    app.update();
    assert!(app.world().get::<Bloom>(other).is_none());
}
