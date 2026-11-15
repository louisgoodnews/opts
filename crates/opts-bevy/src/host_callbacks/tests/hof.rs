//! The HOF callbacks: the destination, the cabin monitor and the route data resolved from the fixture.

use super::*;

#[test]
fn host_callbacks_resolve_hof_data() {
    let hof = Arc::new(HofFile::parse_str(DEMO_HOF).expect("parse demo hof"));
    let route = crate::route::RouteHandle::new(Some(hof));
    route.select(3301).expect("select 3301");
    route.set_stop(1);
    route.set_delay(45.0);
    let ctx = VehicleHostContext {
        seed: 42,
        route,
        ..Default::default()
    };
    let mut h = vehicle_callbacks_with_context(ctx);

    // Terminus callbacks
    let mut st = Stacks::default();
    st.numbers.push(11.0);
    assert!(h("getterminusindex", &mut st));
    assert_eq!(st.numbers.pop(), Some(0.0));

    st.numbers.push(0.0); // termIdx
    st.numbers.push(1.0); // col 1
    assert!(h("getterminusstring", &mut st));
    assert_eq!(st.strings.pop(), Some("Vaals Flats".to_string()));

    st.numbers.push(0.0);
    assert!(h("getterminuscode", &mut st));
    assert_eq!(st.numbers.pop(), Some(11.0));

    // Route callbacks
    st.numbers.push(3301.0);
    assert!(h("getrouteindex", &mut st));
    assert_eq!(st.numbers.pop(), Some(0.0));

    st.numbers.push(0.0); // routeIdx
    st.numbers.push(1.0); // stopNo 1
    assert!(h("getroutebusstopident", &mut st));
    assert_eq!(st.strings.pop(), Some("Ponttor".to_string()));

    st.numbers.push(0.0); // routeIdx
    assert!(h("getrouteterminusindex", &mut st));
    assert_eq!(st.numbers.pop(), Some(0.0)); // terminus index for code 11

    st.numbers.push(0.0);
    assert!(h("getbusstopcount", &mut st));
    assert_eq!(st.numbers.pop(), Some(2.0));

    // Bus stop callbacks
    st.strings.push("ponttor".to_string());
    assert!(h("getbusstopindex", &mut st));
    assert_eq!(st.numbers.pop(), Some(0.0));

    st.numbers.push(0.0); // stopIdx
    st.numbers.push(1.0); // col 1
    assert!(h("getbusstopstring", &mut st));
    assert_eq!(st.strings.pop(), Some("Ponttor".to_string()));

    // Depot global strings
    st.numbers.push(0.0);
    assert!(h("getdepotstringglobal", &mut st));
    assert_eq!(st.strings.pop(), Some("Aachen_Announcements".to_string()));

    // Timetable callbacks
    assert!(h("getttlinestring", &mut st));
    assert_eq!(st.strings.pop(), Some("33".to_string()));

    assert!(h("getttterminusindex", &mut st));
    assert_eq!(st.numbers.pop(), Some(0.0));

    assert!(h("getttbusstopcount", &mut st));
    assert_eq!(st.numbers.pop(), Some(2.0));

    assert!(h("getttbusstopindex", &mut st));
    assert_eq!(st.numbers.pop(), Some(1.0));

    st.numbers.push(0.0);
    assert!(h("getttbusstopname", &mut st));
    assert_eq!(st.strings.pop(), Some("Bushof".to_string()));

    assert!(h("getttdelay", &mut st));
    assert_eq!(st.numbers.pop(), Some(45.0));

    // Tickets & Passengers & Geometry
    st.numbers.push(0.0);
    assert!(h("getticketvalue", &mut st));
    assert_eq!(st.numbers.pop(), Some(2.50));

    st.numbers.push(0.0);
    assert!(h("getticketname", &mut st));
    assert_eq!(st.strings.pop(), Some("Normalfahrschein".to_string()));

    st.numbers.push(1.0);
    assert!(h("givechangecoin", &mut st));

    st.numbers.push(3.0);
    assert!(h("gethumancountonseat", &mut st));
    assert_eq!(st.numbers.pop(), Some(0.0));

    st.numbers.push(1.0);
    st.numbers.push(2.0);
    st.numbers.push(3.0);
    assert!(h("getheightabovepoint", &mut st));
    assert_eq!(st.numbers.pop(), Some(0.0));
}
