//! Unit tests for the overlay info panel: the short page facts and the no-vehicle message.
use super::*;

fn facts() -> PanelFacts<'static> {
    PanelFacts {
        fps: Some((59.6, 21.0)),
        clock: Some("12:34:56".to_owned()),
        vehicle: Some(([892125.9, 5849620.8], 33.3, 306.6, 12.0)),
        session: Some([100.0, 200.0]),
        traffic: None,
        scheduled: None,
        stops: vec![(1234, [130.0, 240.0]), (99, [900.0, 900.0])],
    }
}

#[test]
fn the_short_page_has_the_frame_rate_the_clock_the_map_position_and_the_nearest_stop() {
    let text = lines(&facts());
    assert!(text.contains("60 fps (worst frame 21 ms)"), "{text}");
    assert!(text.contains("clock 12:34:56"), "{text}");
    assert!(
        text.contains("east 892125.9 north 5849620.8 height 33.3"),
        "{text}"
    );
    assert!(
        text.contains("heading 307°") && text.contains("12 km/h"),
        "{text}"
    );
    assert!(text.contains("nearest bus stop: 1234 at 50 m"), "{text}");
}

#[test]
fn without_a_vehicle_the_page_says_so() {
    let mut f = facts();
    f.vehicle = None;
    assert!(lines(&f).ends_with("vehicle: none"));
}
