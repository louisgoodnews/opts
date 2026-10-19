//! Unit tests of `host_callbacks.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

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

use super::*;

const DEMO_HOF: &str = r#"
[name]
Aachen Demo

[servicetrip]
Betriebsfahrt

[global_strings]
2
Aachen_Announcements
Aachen_Rollband

stringcount_terminus
4

[addterminus_list]
	11	Vaals	VAALS	Vaals Flats	VAALS	Vaals
	22	Bushof	BUSHOF	Aachen Bushof	BUSHOF	Bushof
[end]

stringcount_busstop
3

[addbusstop_list]
Ponttor	PONTTOR	Ponttor	Ponttor
Bushof	BUSHOF	Bushof	Bushof
[end]

[infosystem_trip]
3301
FUCHSERDE - VAALS
11
33

[infosystem_busstop_list]
2
Bushof
Ponttor
"#;

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
#[test]
fn host_callbacks_script_textures_lifecycle_and_drawing() {
    let canvas = CpuScriptTexture::new(64, 32);
    let textures = Arc::new(RwLock::new(vec![canvas]));
    let ctx = VehicleHostContext {
        script_textures: textures.clone(),
        ..Default::default()
    };
    let mut h = vehicle_callbacks_with_context(ctx);
    let mut st = Stacks::default();

    // 1. Lock
    st.numbers.push(0.0);
    assert!(h("stlock", &mut st));
    assert!(textures.read().unwrap()[0].is_locked());

    // 2. Set color: A=255, R=255, G=128, B=64
    st.numbers.push(0.0); // tex
    st.numbers.push(255.0); // A
    st.numbers.push(255.0); // R
    st.numbers.push(128.0); // G
    st.numbers.push(64.0); // B
    assert!(h("stsetcolor", &mut st));

    // 3. Draw rect
    st.numbers.push(0.0); // tex
    st.numbers.push(2.0); // x1
    st.numbers.push(2.0); // y1
    st.numbers.push(10.0); // x2
    st.numbers.push(10.0); // y2
    assert!(h("stdrawrect", &mut st));

    // 4. Read pixel
    st.numbers.push(0.0); // tex
    st.numbers.push(5.0); // x
    st.numbers.push(5.0); // y
    assert!(h("streadpixel", &mut st));

    // 5. Get R, G, B, A
    st.numbers.push(0.0);
    assert!(h("stgetr", &mut st));
    assert_eq!(st.numbers.pop(), Some(255.0));

    st.numbers.push(0.0);
    assert!(h("stgetg", &mut st));
    assert_eq!(st.numbers.pop(), Some(128.0));

    st.numbers.push(0.0);
    assert!(h("stgetb", &mut st));
    assert_eq!(st.numbers.pop(), Some(64.0));

    st.numbers.push(0.0);
    assert!(h("stgeta", &mut st));
    assert_eq!(st.numbers.pop(), Some(255.0));

    // 6. Unlock
    st.numbers.push(0.0);
    assert!(h("stunlock", &mut st));
    assert!(!textures.read().unwrap()[0].is_locked());
    assert!(textures.read().unwrap()[0].is_dirty());
}
