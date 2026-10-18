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

#[test]
fn selection_changes_what_the_routes_say_and_unknown_routes_are_refused() {
    let h = handle();
    assert_eq!(
        h.select(1).expect_err("unknown"),
        RouteError::UnknownRoute(1)
    );
    assert!(h.selection().is_none());
    let s = h.select(3301).expect("known");
    assert_eq!((s.line, s.course, s.terminus_code), (33, 1, 22));
    // A depot that marks "no line text" with X shows the line number.
    assert_eq!(s.line_string, "33");
    h.clear();
    assert!(h.selection().is_none());
    assert_eq!(
        RouteHandle::new(None).select(3301).expect_err("no depot"),
        RouteError::NoDepot
    );
}

#[test]
fn the_feed_carries_the_terminus_row_and_the_schedule_flag() {
    let h = handle();
    let mut inputs = HostInputs::default();
    feed_route(&mut inputs, &h);
    assert_eq!(inputs.ai_target_index, -1.0);
    assert!(!inputs.schedule_active);
    h.select(3301).expect("known");
    feed_route(&mut inputs, &h);
    assert_eq!(inputs.ai_target_index, 1.0, "terminus 22 is the second row");
    assert!(inputs.schedule_active);
}

/// A display that ignores the trigger for its first three frames (switching on).
const BUSY_DISPLAY: &str = "
{frame}
(L.L.age) 1 + (S.L.age)
{end}
{trigger:ai_scheduled_settarget}
(L.L.age) 3 >
{if}
(L.L.ai_target_index) (S.L.target_index_int)
{endif}
{end}
";

#[test]
fn the_target_is_offered_again_until_the_display_takes_it() {
    let h = handle();
    h.select(3301).expect("known");
    let (mut engine, d) = ScriptEngine::from_sources(&[("t.osc", BUSY_DISPLAY)]);
    assert!(d.is_empty(), "{d:?}");
    engine.run_init();
    let mut pending: f32 = 0.0;
    assert!(
        schedule_target(&mut engine, &h),
        "a script answers the trigger"
    );
    pending = pending.max(TARGET_OFFER_S);
    engine.set("target_index_int", -1.0);
    for _ in 0..10 {
        let mut inputs = HostInputs::default();
        feed_route(&mut inputs, &h);
        engine.set("ai_target_index", inputs.ai_target_index);
        engine.run_frame(1.0 / 60.0, false);
        keep_target(&mut engine, &h, &mut pending, 1.0 / 60.0);
    }
    assert_eq!(
        engine.get("target_index_int"),
        1.0,
        "taken once it was ready"
    );
    assert_eq!(
        pending, 0.0,
        "the offer ends when the display shows the target"
    );
}
