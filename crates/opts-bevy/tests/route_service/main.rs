//! The route service (M-0099): selection, what the host feeds from it every frame, and the
//! repeated offer of the target to a display that is busy.

use std::sync::Arc;

use opts_bevy::route::{
    feed_route, keep_target, schedule_target, RouteError, RouteHandle, TARGET_OFFER_S,
};
use opts_core::formats::hof::HofFile;
use opts_sim::script::inputs::HostInputs;
use opts_sim::script::ScriptEngine;

const HOF: &str = "
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
	22	Bushof
[end]

stringcount_busstop
1

[addbusstop_list]
Ponttor
[end]

[infosystem_trip]
3301
A - B
22
XXX

[infosystem_busstop_list]
1
Ponttor
";

fn handle() -> RouteHandle {
    RouteHandle::new(Some(Arc::new(HofFile::parse_str(HOF).expect("hof"))))
}

mod depots;
mod feed;
mod offer;
mod selection;
mod timetable;
