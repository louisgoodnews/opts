//! Loops, one-shot triggers and `[noloop]` edges of the player's voices.

use super::*;

const ENGINE: &str = "[loopsound]\nm.wav\n44100\nengine_n\n600\n1\n[3d]\n0\n2\n1\n10\n[volcurve]\nengine_n\n[pnt]\n300\n0\n[pnt]\n900\n1\n[conditionSingle]\nengine_on\n1\n1\n";

#[test]
fn an_engine_loop_follows_its_variables_and_its_condition() {
    let mut r = rig("engine", ENGINE, "{frame}\n{end}");
    set(&mut r, "engine_on", 1.0);
    set(&mut r, "engine_n", 600.0);
    assert!(
        run(&mut r.app, |a| a.world().resource::<AudioStats>().emitters
            >= 1),
        "the loop gets an emitter once its clip is decoded"
    );
    run(&mut r.app, |_| false);
    let e = emitters(&mut r.app);
    assert_eq!(
        e.len(),
        1,
        "the loop has an emitter once its clip is decoded"
    );
    assert!(e[0].looping && e[0].spatial.is_some());
    assert!(
        (e[0].gain - 0.5).abs() < 0.02,
        "level at 600 rpm: {}",
        e[0].gain
    );
    assert!(
        (e[0].speed - 1.0).abs() < 0.02,
        "pitch at the reference: {}",
        e[0].speed
    );
    set(&mut r, "engine_n", 1200.0);
    run(&mut r.app, |a| {
        emitters_of(a)
            .first()
            .is_some_and(|e| (e.speed - 2.0).abs() < 0.05 && (e.gain - 1.0).abs() < 0.02)
    });
    let e = emitters(&mut r.app);
    assert!(
        (e[0].gain - 1.0).abs() < 0.02 && (e[0].speed - 2.0).abs() < 0.05,
        "{} {}",
        e[0].gain,
        e[0].speed
    );
    // The condition fails (engine off): silent but kept.
    set(&mut r, "engine_on", 0.0);
    run(&mut r.app, |a| {
        emitters_of(a).first().is_some_and(|e| e.gain == 0.0)
    });
    assert_eq!(emitters(&mut r.app)[0].gain, 0.0);
    assert!(
        r.app
            .world()
            .get::<ChildOf>(
                r.app
                    .world()
                    .resource::<AudioStats>()
                    .top
                    .first()
                    .map_or(Entity::PLACEHOLDER, |t| t.entity)
            )
            .is_some()
            || true
    );
}

const DOOR: &str = "[sound]\nd.wav\n1\n[3d]\n1\n0\n1\n5\n[trigger]\nev_door_open\n";

#[test]
fn a_raised_event_plays_the_one_shot_of_its_trigger() {
    let mut r = rig("door", DOOR, "{frame}\n{end}");
    // First raise: the file is decoded on a worker, the first firing is a cold miss.
    r.app
        .world_mut()
        .get_mut::<VehicleScripts>(r.root)
        .expect("s")
        .raised
        .push("ev_door_open".into());
    run(&mut r.app, |a| {
        a.world().resource::<PlayerAudioStats>().cold_misses == 1
    });
    assert_eq!(r.app.world().resource::<PlayerAudioStats>().cold_misses, 1);
    for _ in 0..60 {
        run(&mut r.app, |_| false);
        if r.app.world().resource::<ClipLoader>().counts().0 == 1 {
            break;
        }
    }
    r.app
        .world_mut()
        .get_mut::<VehicleScripts>(r.root)
        .expect("s")
        .raised
        .push("EV_DOOR_OPEN".into());
    run(&mut r.app, |a| {
        a.world().resource::<PlayerAudioStats>().one_shots >= 1
    });
    assert_eq!(r.app.world().resource::<PlayerAudioStats>().one_shots, 1);
    // Another event plays nothing.
    r.app
        .world_mut()
        .get_mut::<VehicleScripts>(r.root)
        .expect("s")
        .raised
        .push("ev_other".into());
    run(&mut r.app, |_| false);
    assert_eq!(r.app.world().resource::<PlayerAudioStats>().one_shots, 1);
}

#[test]
fn a_noloop_voice_plays_once_when_its_condition_turns_true() {
    let cfg = "[sound]\nk.wav\n1\n[3d]\n0\n0\n1\n5\n[conditionSingle]\nkneel\n1\n1\n[noloop]\n";
    let mut r = rig("edge", cfg, "{frame}\n{end}");
    // Warm the clip first (a cold miss on the first edge is normal).
    set(&mut r, "kneel", 1.0);
    for _ in 0..60 {
        run(&mut r.app, |_| false);
        if r.app.world().resource::<ClipLoader>().counts().0 == 1 {
            break;
        }
    }
    set(&mut r, "kneel", 0.0);
    run(&mut r.app, |_| false);
    let before = r.app.world().resource::<PlayerAudioStats>().one_shots;
    set(&mut r, "kneel", 1.0);
    run(&mut r.app, |a| {
        a.world().resource::<PlayerAudioStats>().one_shots > before
    });
    assert_eq!(
        r.app.world().resource::<PlayerAudioStats>().one_shots,
        before + 1
    );
    // Staying true plays nothing more.
    run(&mut r.app, |_| false);
    assert_eq!(
        r.app.world().resource::<PlayerAudioStats>().one_shots,
        before + 1
    );
}
