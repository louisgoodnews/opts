//! Applying the environment and resuming after a rebuild.


use super::*;
use opts_bevy::{CalendarDate, ChronoRebuild, MapDateChanged};


#[test]
fn apply_sets_the_date_and_announces_the_map_date() {
    let mut app = drive_at_noon();
    open_page(&mut app);
    click(&mut app, "pause.environment.day.plus");
    click(&mut app, "pause.environment.year.minus");
    click(&mut app, "pause.environment.minute.plus");
    click(&mut app, "pause.environment.apply");
    let e = env(&app);
    assert_eq!(e.date(), CalendarDate::new(1993, 6, 2));
    assert_eq!(e.map_date(), CalendarDate::new(1993, 6, 2));
    assert_eq!(e.hms(), (12, 5, 0));
    let msgs = app.world().resource::<Messages<MapDateChanged>>();
    let mut cursor = msgs.get_cursor();
    let got: Vec<_> = cursor.read(msgs).copied().collect();
    assert_eq!(
        got,
        vec![MapDateChanged {
            old: CalendarDate::new(1994, 6, 1),
            new: CalendarDate::new(1993, 6, 2)
        }]
    );
    assert!(ids(&mut app).contains(&"pause.resume".to_string()));
}

#[test]
fn resume_waits_for_a_map_rebuild() {
    let mut app = drive_at_noon();
    esc(&mut app);
    app.world_mut()
        .resource_mut::<ChronoRebuild>()
        .pending_tiles = 3;
    click(&mut app, "pause.resume");
    assert!(!app.world().resource::<Pause>().open);
    assert!(paused(&app), "still rebuilding");
    app.world_mut()
        .resource_mut::<ChronoRebuild>()
        .pending_tiles = 0;
    app.update();
    assert!(!paused(&app));
}
