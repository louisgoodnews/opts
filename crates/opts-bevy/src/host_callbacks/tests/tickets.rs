//! The ticket callbacks and the human count callbacks.

use super::*;

#[test]
fn the_ticket_callbacks_answer_from_the_maps_pack_and_the_change_is_logged() {
    use opts_sim::passengers::{Ticket, TicketBook};
    let ticket = |display: &str, value: f32| Ticket {
        display: display.into(),
        value,
        age_min: 15,
        age_max: 200,
        max_stations: 0,
        probability: 1.0,
        day_ticket: false,
    };
    let ctx = VehicleHostContext {
        tickets: std::sync::Arc::new(TicketBook {
            tickets: vec![ticket("Kurz@strecke", 1.10), ticket("Einzel", 2.25)],
            ..Default::default()
        }),
        coins: std::sync::Arc::new(vec![0.10, 0.50]),
        ..Default::default()
    };
    let log = ctx.change_given.clone();
    let mut h = vehicle_callbacks_with_context(ctx);
    let mut st = Stacks::default();
    st.numbers.push(1.0);
    assert!(h("getticketvalue", &mut st));
    assert_eq!(st.numbers.pop(), Some(2.25));
    st.numbers.push(0.0);
    assert!(h("getticketname", &mut st));
    assert_eq!(
        st.strings.pop(),
        Some("Kurz strecke".to_string()),
        "`@` is a line break"
    );
    // A ticket the pack does not have: the stub answers, as before.
    st.numbers.push(9.0);
    assert!(h("getticketvalue", &mut st));
    assert_eq!(st.numbers.pop(), Some(2.5));
    // Change: coin 1 is 0.50; a coin that does not exist is not logged.
    for coin in [1.0, 0.0, 7.0] {
        st.numbers.push(coin);
        assert!(h("givechangecoin", &mut st));
    }
    assert_eq!(*log.read().expect("log"), vec![0.5, 0.1]);
}

#[test]
fn the_human_count_callbacks_read_the_passenger_counts() {
    let counts = std::sync::Arc::new(std::sync::RwLock::new(PassengerCounts {
        on_place: vec![0, 2, 1],
        on_link: vec![3],
    }));
    let ctx = VehicleHostContext {
        passengers: counts.clone(),
        ..Default::default()
    };
    let mut h = vehicle_callbacks_with_context(ctx);
    let mut st = Stacks::default();
    let ask = |h: &mut opts_sim::script::ExternalHandler, st: &mut Stacks, name: &str, i: f32| {
        st.numbers.push(i);
        assert!(h(name, st));
        st.numbers.pop().expect("answer")
    };
    assert_eq!(ask(&mut h, &mut st, "gethumancountonseat", 1.0), 2.0);
    assert_eq!(
        ask(&mut h, &mut st, "gethumancountonseat", 9.0),
        0.0,
        "no such place"
    );
    assert_eq!(ask(&mut h, &mut st, "gethumancountonpathlink", 0.0), 3.0);
    counts.write().expect("w").on_place[0] = 5;
    assert_eq!(
        ask(&mut h, &mut st, "gethumancountonseat", 0.0),
        5.0,
        "live"
    );
}
