//! The repeated offer of the target to a display that is busy.

use super::*;

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
