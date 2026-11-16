//! Timetable runs: the callbacks and the delay of a followed run.

use super::*;
use opts_bevy::host_callbacks::{vehicle_callbacks_with_context, VehicleHostContext};
use opts_core::formats::timetable::{parse_busstops, parse_line, parse_trip, Timetables};
use opts_sim::script::Stacks;

const TTP: &str = "[trip]\n\nVaals\n33\n[station_typ2]\n100\n[station_typ2]\n101\n[station_typ2]\n102\n[profile]\nstandard\n10.000\n[profile_man_arr_time]\n0\n0.0\n[profile_man_arr_time]\n1\n4.0\n[profile_man_arr_time]\n2\n10.0\n";
const TTL: &str =
    "[userallowed]\n[priority]\n1\n[newtour]\nT1\nBus\n1\n[addtrip]\n33_V\n0\n480.000\n";
const STOPS: &str = "[busstop]\nPonttor\n1\n100\n0\n0\n\n[busstop]\nBushof\n1\n101\n0\n0\n\n[busstop]\nVaals\n1\n102\n0\n0\n\n";

fn handle() -> RouteHandle {
    let h = handle_with_depot();
    h.set_timetables(Some(Arc::new(Timetables::from_parts(
        parse_busstops(STOPS.as_bytes()),
        vec![("33".into(), parse_line(TTL.as_bytes()))],
        vec![("33_V".into(), parse_trip(TTP.as_bytes()))],
    ))));
    h
}

fn handle_with_depot() -> RouteHandle {
    // The depot of the module above, with a terminus whose text is the trip's destination
    // and a route of line 33 to it.
    let hof = "
[name]
Depot

[servicetrip]
x

[global_strings]
1
a

stringcount_terminus
1

[addterminus_list]
	11	Vaals
[end]

stringcount_busstop
1

[addbusstop_list]
Ponttor
[end]

[infosystem_trip]
3301
A - B
11
33

[infosystem_busstop_list]
1
Ponttor
";
    RouteHandle::new(Some(Arc::new(HofFile::parse_str(hof).expect("hof"))))
}

fn call(h: &mut opts_sim::script::ExternalHandler, name: &str, args: &[f32]) -> Stacks {
    let mut st = Stacks::default();
    st.numbers.extend_from_slice(args);
    assert!(h(name, &mut st), "{name}");
    st
}

mod callbacks;
mod delay;
