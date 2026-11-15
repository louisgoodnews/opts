//! Unit tests of `datetime_select`.

use super::*;

#[test]
fn test_datetime_select_event_emission() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<DateTimeSelected>()
        .add_systems(Update, datetime_select_system);

    let entity = {
        let mut commands = app.world_mut().commands();
        DateTimeSelectBuilder::new(2026, 10, 8, 14, 30).spawn(&mut commands)
    };

    app.update();

    // Mutate the datetime
    if let Some(mut dt) = app.world_mut().get_mut::<M3DateTimeSelect>(entity) {
        dt.minute = 45;
    }

    app.update();

    let events = app.world().resource::<Messages<DateTimeSelected>>();
    let mut reader = events.get_cursor();
    let received = reader.read(events).filter(|e| e.entity == entity).last();
    assert_eq!(
        received.map(|e| (e.year, e.month, e.day, e.hour, e.minute)),
        Some((2026, 10, 8, 14, 45))
    );
}
