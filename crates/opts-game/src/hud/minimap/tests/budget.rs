//! The redraw's time budget.

use super::*;

#[test]
fn a_redraw_stays_within_the_budget() {
    // A dense city block at a large texture: 800 chords of about 40 px and a few vehicles (about
    // 150 000 pixel visits). The fastest of many runs is taken: the minimum is robust against a
    // busy machine.
    //
    // The 1 ms budget is for the release build. The dev profile compiles this crate at
    // opt-level 1 (only dependencies get 3), which costs about 2x on this loop (measured: about
    // 2.2 ms here, 14 ms at opt-level 0), so a debug-assertions build gets twice the slack, like
    // the other timing tests in the repo that allow CI runners a wide margin. The release time
    // is what `hud.state` reports live as minimap.last_us.
    let c = colors();
    let mut lines = Vec::new();
    for i in 0..400 {
        let o = (i % 40) as f32 * 5.0 - 100.0;
        let s = (i / 40) as f32 * 20.0 - 100.0;
        lines.push(line_w((o, s), (o + 2.0, s + 18.0), 3.5));
        lines.push(line_w((s, o), (s + 18.0, o + 3.0), 3.5));
    }
    let markers: Vec<MapMarker> = (0..6)
        .map(|i| MapMarker {
            pos: Vec2::new(i as f32 * 20.0 - 50.0, 10.0),
            heading: i as f32,
            active: i == 0,
        })
        .collect();
    let mut canvas = Canvas::new(224, c.ground);
    let mut best = u128::MAX;
    for _ in 0..100 {
        let t = std::time::Instant::now();
        render(&mut canvas, NORTH_UP, &lines, &markers, &c);
        best = best.min(t.elapsed().as_micros());
    }
    let slack: u128 = if cfg!(debug_assertions) { 6 } else { 3 };
    assert!(
        best < u128::from(BUDGET_US) * slack,
        "{best} us for 800 chords at 224 px (budget {BUDGET_US} us, {slack}x slack for a busy CI \
         machine and the dev profile; the live time is `hud.state` minimap.last_us)"
    );
}
