//! Bus tests: one-shots that clean up after themselves, and the bus settings.

use super::*;


#[test]
fn one_shots_clean_up_after_themselves() {
    let mut app = app();
    let c = clip(&mut app);
    // Not heard when triggered: gone at once.
    let far = emitter_at(
        &mut app,
        900.0,
        SoundEmitter::new(c.clone(), SoundBus::Effects).spatial(Distance::new(5.0, 100.0)),
    );
    // Heard: plays, and goes when Bevy removes the finished player.
    let heard = emitter_at(
        &mut app,
        3.0,
        SoundEmitter::new(c, SoundBus::Effects).spatial(Distance::new(5.0, 100.0)),
    );
    app.update();
    app.update();
    assert!(app.world().get_entity(far).is_err());
    assert!(has_voice(&app, heard));
    app.world_mut()
        .entity_mut(heard)
        .remove::<AudioPlayer<PcmClip>>();
    app.update();
    app.update();
    assert!(app.world().get_entity(heard).is_err());
}

#[test]
fn settings_scale_and_silence_the_buses() {
    let mut app = app();
    let c = clip(&mut app);
    let e = emitter_at(
        &mut app,
        1.0,
        SoundEmitter::new(c, SoundBus::Vehicles).looping(),
    );
    app.world_mut().resource_mut::<AudioSettings>().vehicles = 50.0;
    app.update();
    app.update();
    let gain = app.world().get::<Voice>(e).map(|v| v.gain).unwrap_or(0.0);
    assert!((gain - 0.64 * 0.25).abs() < 1e-3, "{gain}");
    app.world_mut().resource_mut::<AudioSettings>().enabled = false;
    app.update();
    app.update();
    assert!(!has_voice(&app, e), "sound off: no voices");
    app.world_mut().resource_mut::<AudioSettings>().enabled = true;
    app.update();
    app.update();
    assert!(has_voice(&app, e));
}
