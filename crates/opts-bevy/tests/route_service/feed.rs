//! What the host feeds from the selected route every frame.

use super::*;

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
