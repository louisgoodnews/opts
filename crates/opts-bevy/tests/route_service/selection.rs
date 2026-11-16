//! Route selection: the known routes are selected, unknown ones refused.

use super::*;

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
