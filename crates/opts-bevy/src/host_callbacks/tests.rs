//! Unit tests of `host_callbacks.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

use opts_core::formats::hof::HofFile;

#[test]
fn aachen_line_33_destination_and_cabin_monitor_resolution() {
    let aachen_path = std::path::Path::new(
        "/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2/Vehicles/MAN_NL_NG/Aachen.hof",
    );
    if !aachen_path.exists() {
        return;
    }
    let bytes = std::fs::read(aachen_path).expect("read Aachen.hof");
    let hof = Arc::new(HofFile::parse_bytes(&bytes).expect("parse Aachen.hof"));
    assert_eq!(hof.name, "Aachen");

    // Route 3301: Line 33 Route 01 (FUCHSERDE >> VAALS)
    let (_, route) = hof.find_route_by_code(3301).expect("find route 3301");
    assert_eq!(route.terminus_code, 11);
    assert!(!route.busstops.is_empty());

    let route = crate::route::RouteHandle::new(Some(hof.clone()));
    route.select(3301).expect("select 3301");
    let ctx = VehicleHostContext {
        seed: 1234,
        route,
        ..Default::default()
    };
    let mut h = vehicle_callbacks_with_context(ctx);

    let mut st = Stacks::default();

    // 1. Matrix display queries active timetable line string
    assert!(h("getttlinestring", &mut st));
    assert_eq!(st.strings.pop(), Some("33".to_string()));

    // 2. Matrix display queries timetable destination terminus index
    assert!(h("getttterminusindex", &mut st));
    let term_idx = st.numbers.pop().expect("term_idx");
    assert!(term_idx >= 0.0);

    // 3. Front matrix queries destination lines
    // string 1: Front-ANNAX upper line
    st.numbers.push(term_idx);
    st.numbers.push(1.0);
    assert!(h("getterminusstring", &mut st));
    assert_eq!(st.strings.pop(), Some("V A A L S*B".to_string()));

    // string 2: Front-ANNAX lower line
    st.numbers.push(term_idx);
    st.numbers.push(2.0);
    assert!(h("getterminusstring", &mut st));
    assert_eq!(st.strings.pop(), Some("FLATS".to_string()));

    // 4. Cabin monitor queries stop names
    assert!(h("getttbusstopcount", &mut st));
    let stop_count = st.numbers.pop().expect("stop_count");
    assert!(stop_count > 0.0);

    st.numbers.push(0.0); // stop 0
    assert!(h("getttbusstopname", &mut st));
    let first_stop = st.strings.pop().expect("first stop name");
    assert!(!first_stop.is_empty());
}

#[test]
fn nr_spec_random_is_stable_per_vehicle_and_number() {
    let mut a = vehicle_callbacks(1);
    let mut b = vehicle_callbacks(2);
    let call = |h: &mut ExternalHandler, n: f32| {
        let mut st = Stacks::default();
        st.numbers.push(n);
        assert!(h("nrspecrandom", &mut st));
        st.numbers[0]
    };
    let (x, y) = (call(&mut a, 301.0), call(&mut a, 302.0));
    assert_eq!(x, call(&mut a, 301.0));
    assert_ne!(x, y);
    assert_ne!(x, call(&mut b, 301.0));
    assert!((0.0..1.0).contains(&x));
    assert!(!a("unknown", &mut Stacks::default()));
}
#[path = "tests/fixture.rs"]
mod fixture;
#[path = "tests/hof.rs"]
mod hof;
#[path = "tests/textures.rs"]
mod textures;
#[path = "tests/tickets.rs"]
mod tickets;

use fixture::*;
